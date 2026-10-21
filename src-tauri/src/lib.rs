// 视频内容提取器 - Tauri 后端

// 模块声明
pub mod ai;
pub mod commands;
pub mod core;
pub mod data;
pub mod utils;

use tauri::Emitter;
use tauri::Manager;

/// 示例命令 - 用于测试
#[tauri::command]
fn greet(name: &str) -> String {
    format!("你好，{}！欢迎使用视频内容提取器！", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化路径管理器和首次运行检测
    if let Err(e) = utils::paths::init_app_paths() {
        eprintln!("路径管理器初始化失败: {}", e);
    }

    // 执行首次运行初始化
    if let Err(e) = utils::paths::FirstRunDetector::initialize_first_run() {
        eprintln!("首次运行初始化失败: {}", e);
    }

    // 初始化日志系统
    utils::logger::init_logger();

    // Windows WebView2 优化：使用固定数据目录缓存 WebView2 状态，减少冷启动时间
    #[cfg(target_os = "windows")]
    {
        // 使用 AppPaths 获取数据目录
        use utils::paths::get_app_paths;

        let webview_data = if let Ok(paths) = get_app_paths() {
            paths.data_dir.join("WebView2")
        } else {
            // 回退到旧的实现
            dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("QiushiKLBilibiliPublic")
                .join("WebView2")
        };

        if std::fs::create_dir_all(&webview_data).is_ok() {
            std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &webview_data);
        }
    }

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            commands::settings::init_data_layer().map_err(|e| {
                let err = std::io::Error::new(std::io::ErrorKind::Other, e);
                Box::new(err) as Box<dyn std::error::Error>
            })?;

            app.manage(core::sidecar_manager::SidecarState::default());

            // 初始化系统托盘
            if let Err(e) = core::tray::init_tray(app.handle()) {
                eprintln!("系统托盘初始化失败: {}", e);
            }

            // 启动 Python Sidecar (dy-mcp API)
            core::sidecar_manager::init_sidecar(app.handle());

            // 为主窗口设置圆角 (Windows 11+)
            #[cfg(target_os = "windows")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    use windows::Win32::Foundation::HWND;
                    use windows::Win32::Graphics::Dwm::{
                        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
                    };

                    if let Ok(hwnd) = window.hwnd() {
                        let preference = DWMWCP_ROUND.0 as u32;
                        unsafe {
                            let _ = DwmSetWindowAttribute(
                                HWND(hwnd.0 as *mut _),
                                DWMWA_WINDOW_CORNER_PREFERENCE,
                                &preference as *const _ as *const _,
                                std::mem::size_of::<u32>() as u32,
                            );
                        }
                    }
                }
            }

            // 监听菜单事件
            let app_handle = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                match event.id().as_ref() {
                    "show" => {
                        core::tray::show_main_window(&app_handle);
                    }
                    "tasks" => {
                        // 发送事件到前端，导航到任务页面
                        let _ = app_handle.emit("navigate", "/tasks");
                        core::tray::show_main_window(&app_handle);
                    }
                    "quit" => {
                        core::sidecar_manager::cleanup_sidecars(&app_handle);
                        app_handle.exit(0);
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 测试命令
            greet,
            // 视频处理命令
            commands::video::is_supported_format,
            commands::video::get_video_info,
            commands::video::get_videos_info,
            commands::video::extract_audio,
            commands::video::extract_audio_to_file,
            commands::video::transcribe_video,
            commands::video::transcribe_videos_batch,
            commands::video::generate_thumbnail,
            commands::video::cleanup_temp_files,
            commands::video::export_transcripts_to_docx,
            commands::video::export_transcripts_to_txt,
            commands::video::download_video,
            commands::video::download_videos_batch,
            // 语音识别命令
            commands::asr::transcribe_audio,
            commands::asr::detect_gpu,
            commands::asr::check_model_exists,
            commands::asr::download_model,
            commands::asr::get_model_status,
            commands::asr::open_models_dir,
            commands::asr::delete_asr_model,
            // B站独立入口，复用已保存的识别设置
            commands::bilibili::parse_bilibili_link,
            commands::bilibili::extract_bilibili_content,
            commands::bilibili::get_bilibili_login_status,
            commands::bilibili::open_bilibili_login,
            commands::bilibili::start_bilibili_creator_collection,
            commands::bilibili::get_bilibili_creator_status,
            commands::bilibili::cancel_bilibili_creator_collection,
            commands::bilibili::download_bilibili_video,
            // MCP 服务命令
            commands::mcp::parse_douyin_link,
            commands::mcp::parse_douyin_links_batch,
            commands::mcp::search_douyin_videos,
            commands::mcp::get_user_videos,
            commands::mcp::collect_creator_page,
            commands::mcp::cancel_creator_collection,
            commands::mcp::resolve_creator_id,
            commands::mcp::update_mcp_config,
            commands::mcp::get_mcp_config,
            commands::mcp::check_dy_mcp_health,
            commands::mcp::check_undoom_mcp_health,
            commands::mcp::validate_douyin_link,
            commands::mcp::extract_douyin_content,
            // AI 分析命令
            commands::ai::analyze_content,
            commands::ai::check_lm_studio,
            commands::ai::init_knowledge_base,
            commands::ai::add_document_to_kb,
            commands::ai::search_knowledge_base,
            commands::ai::delete_document_from_kb,
            commands::ai::list_documents,
            commands::ai::export_knowledge_base,
            commands::ai::import_knowledge_base,
            commands::ai::clear_knowledge_base,
            commands::ai::update_ai_settings,
            commands::ai::get_ai_settings,
            commands::ai::get_embedding_model_status,
            commands::ai::download_embedding_model,
            commands::ai::delete_embedding_model,
            commands::ai::open_embedding_models_dir,
            commands::ai::chat_with_ai,
            commands::agent::list_agent_skills,
            commands::agent::run_agent_skill,
            // 设置命令
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::reset_settings,
            commands::settings::select_directory,
            commands::settings::get_logs,
            commands::settings::clear_app_logs,
            commands::settings::get_log_files,
            commands::settings::open_logs_dir,
            commands::settings::check_agreement_accepted,
            commands::settings::save_agreement_accepted,
            commands::settings::exit_app,
            // GPU 命令
            commands::gpu::detect_gpu_info,
            commands::gpu::get_recommended_gpu_config,
            commands::gpu::validate_gpu_config,
            commands::gpu::save_gpu_device_id,
            // 任务队列命令
            commands::task_queue::add_transcription_task,
            commands::task_queue::add_link_parsing_task,
            commands::task_queue::add_download_task,
            commands::task_queue::add_analysis_task,
            commands::task_queue::pause_task,
            commands::task_queue::resume_task,
            commands::task_queue::cancel_task,
            commands::task_queue::get_task_status,
            commands::task_queue::get_task_info,
            commands::task_queue::list_pending_tasks,
            commands::task_queue::list_task_history,
            commands::task_queue::list_all_tasks,
            commands::task_queue::get_queue_stats,
            commands::task_queue::get_current_task,
            commands::task_queue::clear_task_history,
            commands::task_queue::clear_pending_tasks,
            // 托盘命令
            commands::tray::show_window,
            commands::tray::hide_to_tray,
            commands::tray::send_notification,
            commands::tray::update_tray_tooltip,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            core::sidecar_manager::cleanup_sidecars(app_handle);
        }
    });
}
