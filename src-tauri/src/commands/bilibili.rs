use crate::core::mcp_client::SIDECAR_BASE_URL;
use crate::utils::ffmpeg::FfmpegWrapper;
use crate::utils::paths::get_app_paths;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;
use tauri::AppHandle;
use uuid::Uuid;

async fn sidecar_request(endpoint: &str, body: Option<Value>, timeout: u64) -> Result<Value, String> {
    let token = crate::core::sidecar_manager::sidecar_token()
        .ok_or_else(|| "采集服务正在启动，请稍后重试".to_string())?;
    let client = reqwest::Client::builder().timeout(Duration::from_secs(timeout))
        .build().map_err(|e| e.to_string())?;
    let url = format!("{}{}", SIDECAR_BASE_URL, endpoint);
    let request = match body {
        Some(body) => client.post(url).json(&body),
        None => client.get(url),
    };
    let response = request.header("X-Sidecar-Token", token).send().await
        .map_err(|_| "B站采集服务连接失败或请求超时，请稍后重试".to_string())?;
    let payload: Value = response.json().await
        .map_err(|_| "B站采集服务返回格式异常".to_string())?;
    if payload["status"] != "success" {
        return Err(payload["error"].as_str().unwrap_or("B站采集失败").to_string());
    }
    Ok(payload["data"].clone())
}

#[tauri::command]
pub async fn parse_bilibili_link(app:AppHandle, url: String) -> Result<Value, String> {
    let queue=crate::commands::task_queue::get_task_queue();
    let task=queue.add_task(crate::data::task_queue::TaskType::LinkParsing {links:vec![url.clone()]}).await;
    let _=queue.start_task_by_id(&task).await;
    let result=sidecar_request("/bilibili/parse", Some(json!({"url": url})), 90).await;
    finish_task(&app,&task,&result,"B站作品信息已读取").await;
    result
}

#[tauri::command]
pub async fn start_bilibili_creator_collection(input: String, target_count: usize) -> Result<Value, String> {
    sidecar_request("/bilibili/creator/start", Some(json!({"input": input, "target_count": target_count})), 60).await
}

#[tauri::command]
pub async fn get_bilibili_creator_status(task_id: String) -> Result<Value, String> {
    if Uuid::parse_str(&task_id).is_err() { return Err("UP主采集任务编号无效".into()); }
    sidecar_request(&format!("/bilibili/creator/status?task_id={}", task_id), None, 20).await
}

#[tauri::command]
pub async fn cancel_bilibili_creator_collection(task_id: Option<String>) -> Result<Value, String> {
    sidecar_request("/bilibili/creator/cancel", Some(json!({"task_id": task_id})), 20).await
}

async fn finish_task<T>(app: &AppHandle, task_id: &str, result: &Result<T, String>, message: &str) {
    let queue = crate::commands::task_queue::get_task_queue();
    match result {
        Ok(_) => {
            queue.complete_task_by_id(task_id, Some(message.to_string())).await;
            crate::commands::task_queue::emit_task_completed(app, task_id, Some(message));
        }
        Err(error) => {
            queue.fail_task_by_id(task_id, error.clone()).await;
            crate::commands::task_queue::emit_task_failed(app, task_id, error);
        }
    }
}

#[tauri::command]
pub async fn download_bilibili_video(app: AppHandle, url: String, output_path: String) -> Result<String, String> {
    use crate::data::task_queue::TaskType;
    let output = PathBuf::from(&output_path);
    if !output.is_absolute() || output.extension().and_then(|v| v.to_str()) != Some("mp4") {
        return Err("请选择绝对保存路径，文件格式为MP4".into());
    }
    if output.exists() { return Err("目标文件已经存在，请更换文件名或保存目录".into()); }
    let queue = crate::commands::task_queue::get_task_queue();
    let task = queue.add_task(TaskType::VideoDownload {
        url: url.clone(), output_path: output_path.clone(),
        video_name: output.file_name().unwrap_or_default().to_string_lossy().to_string(),
    }).await;
    let _ = queue.start_task_by_id(&task).await;
    let result = async {
        let paths = get_app_paths().map_err(|e| e.to_string())?;
        let temp = paths.temp_dir.join(format!("bilibili_video_{}.mp4", Uuid::new_v4().simple()));
        let _cleanup = TemporaryFiles(vec![temp.clone()]);
        sidecar_request("/bilibili/download", Some(json!({"url": url, "path": temp})), 3600).await?;
        if std::fs::metadata(&temp).map_err(|e| e.to_string())?.len() == 0 {
            return Err("B站返回了空视频文件".into());
        }
        let parent = output.parent().ok_or("保存路径无效")?;
        tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
        // create_new protects an existing user file even if it appeared during download.
        let mut target = tokio::fs::OpenOptions::new().write(true).create_new(true)
            .open(&output).await.map_err(|_| "文件已存在或目录不可写，请更换保存位置".to_string())?;
        let mut source = tokio::fs::File::open(&temp).await.map_err(|e| e.to_string())?;
        if let Err(error) = tokio::io::copy(&mut source, &mut target).await {
            drop(target);
            let _ = tokio::fs::remove_file(&output).await;
            return Err(format!("保存B站视频失败: {}", error));
        }
        target.sync_all().await.map_err(|e| e.to_string())?;
        Ok(output_path)
    }.await;
    finish_task(&app, &task, &result, "B站视频已下载，包含声音和画面").await;
    result
}

#[tauri::command]
pub async fn get_bilibili_login_status() -> Result<Value, String> {
    sidecar_request("/bilibili/login/status", None, 15).await
}

#[tauri::command]
pub async fn open_bilibili_login() -> Result<Value, String> {
    sidecar_request("/bilibili/login", Some(json!({})), 15).await
}

#[derive(Debug, Deserialize)]
struct PreparedTranscript {
    source: String,
    transcript: Option<String>,
    audio_path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct BilibiliTranscript {
    pub text: String,
    pub source: String,
}

struct TemporaryFiles(Vec<PathBuf>);
impl Drop for TemporaryFiles {
    fn drop(&mut self) {
        for path in &self.0 { let _ = std::fs::remove_file(path); }
    }
}

#[tauri::command]
pub async fn extract_bilibili_content(app: AppHandle, url: String, title:Option<String>) -> Result<BilibiliTranscript, String> {
    use crate::data::task_queue::TaskType;
    let queue = crate::commands::task_queue::get_task_queue();
    let task = queue.add_task(TaskType::PlatformTranscription {
        platform: "B站".into(), source_url: url.clone(), title: title.unwrap_or_else(||url.clone()),
    }).await;
    let _ = queue.start_task_by_id(&task).await;
    let result = extract_bilibili_content_inner(app.clone(), url).await;
    finish_task(&app, &task, &result, "B站文案已提取").await;
    result
}

async fn extract_bilibili_content_inner(app: AppHandle, url: String) -> Result<BilibiliTranscript, String> {
    let paths = get_app_paths().map_err(|e| e.to_string())?;
    let output = paths.temp_dir.join(format!("bilibili_{}.audio", Uuid::new_v4().simple()));
    let prepared = sidecar_request("/bilibili/prepare", Some(json!({"url": url, "path": output})), 1800).await?;
    let prepared: PreparedTranscript = serde_json::from_value(prepared).map_err(|_| "B站音轨响应格式异常".to_string())?;
    if prepared.source == "subtitle" {
        let text = prepared.transcript.filter(|v| !v.trim().is_empty()).ok_or("B站字幕为空")?;
        return Ok(BilibiliTranscript { text, source: "subtitle".into() });
    }
    let audio = prepared.audio_path.ok_or("未获得B站音轨")?;
    let resolved = audio.canonicalize().map_err(|_| "B站音轨不存在".to_string())?;
    let allowed = paths.temp_dir.canonicalize().map_err(|e| e.to_string())?;
    if !resolved.starts_with(&allowed) || audio.file_stem() != output.file_stem() {
        return Err("B站音轨路径不在本次任务的临时目录内".into());
    }
    let wav = output.with_extension("wav");
    let _cleanup = TemporaryFiles(vec![audio.clone(), wav.clone()]);
    let wav_for_worker = wav.clone();
    tokio::task::spawn_blocking(move || {
        FfmpegWrapper::new().and_then(|ffmpeg| ffmpeg.extract_audio_for_asr(&audio, &wav_for_worker))
            .map_err(|e| format!("B站音频转换失败: {}", e))
    }).await.map_err(|e| e.to_string())??;
    let text = super::mcp::transcribe_audio_with_current_settings(app, &wav).await?;
    if text.trim().is_empty() { return Err("语音识别未返回文案，请检查视频是否含有语音".into()); }
    Ok(BilibiliTranscript { text, source: "audio".into() })
}
