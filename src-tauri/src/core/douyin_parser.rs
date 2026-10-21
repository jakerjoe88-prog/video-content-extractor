// 抖音视频解析器 - 纯 Rust 实现
// 通过解析网页获取视频信息，无需外部 Python 服务
#![allow(dead_code)]

use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum DouyinError {
    #[error("网络请求失败: {0}")]
    NetworkError(String),
    #[error("解析失败: {0}")]
    ParseError(String),
    #[error("无效的抖音链接: {0}")]
    InvalidLink(String),
    #[error("请求超时")]
    Timeout,
    #[error("视频不存在或已删除")]
    VideoNotFound,
    #[error("需要登录或验证")]
    AuthRequired,
}

/// 抖音视频信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DouyinVideoData {
    pub aweme_id: String,
    pub video_url: String,
    pub no_watermark_url: String,
    pub title: String,
    pub author: String,
    pub author_id: String,
    pub likes: u64,
    pub comments: u64,
    pub shares: u64,
    pub cover_url: String,
    pub duration: u64,
    pub create_time: u64,
}

/// 抖音解析器
pub struct DouyinParser {
    client: Client,
}

impl DouyinParser {
    pub fn new() -> Self {
        // 使用更简单的配置，避免 TLS 相关问题
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .danger_accept_invalid_certs(false)
            .build()
            .unwrap_or_else(|_| {
                // 如果构建失败，使用默认客户端
                Client::new()
            });

        Self { client }
    }

    /// 创建带有自定义配置的解析器
    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// 解析抖音链接获取视频信息
    pub async fn parse_link(&self, link: &str) -> Result<DouyinVideoData, DouyinError> {
        let link = link.trim();

        // 验证链接格式
        if !Self::is_valid_link(link) {
            return Err(DouyinError::InvalidLink(link.to_string()));
        }

        // 1. 获取重定向后的实际 URL
        let final_url = self.follow_redirect(link).await?;

        // 2. 从 URL 中提取视频 ID
        let aweme_id = self.extract_aweme_id(&final_url)?;

        // 3. 通过网页解析获取视频详情（避免 API 反爬）
        self.fetch_video_from_page(&aweme_id).await
    }

    /// 跟随重定向获取最终 URL
    async fn follow_redirect(&self, url: &str) -> Result<String, DouyinError> {
        let response = self
            .client
            .get(url)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DouyinError::Timeout
                } else {
                    DouyinError::NetworkError(e.to_string())
                }
            })?;

        Ok(response.url().to_string())
    }

    /// 从 URL 中提取视频 ID (aweme_id)
    fn extract_aweme_id(&self, url: &str) -> Result<String, DouyinError> {
        // 匹配模式: /video/123456789 或 /note/123456789
        let re = Regex::new(r"/(?:video|note)/(\d+)").unwrap();

        if let Some(caps) = re.captures(url) {
            if let Some(id) = caps.get(1) {
                return Ok(id.as_str().to_string());
            }
        }

        // 尝试从查询参数中提取
        if let Some(pos) = url.find("modal_id=") {
            let start = pos + 9;
            let end = url[start..]
                .find('&')
                .map(|i| start + i)
                .unwrap_or(url.len());
            let id = &url[start..end];
            if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
                return Ok(id.to_string());
            }
        }

        Err(DouyinError::ParseError(format!(
            "无法从 URL 中提取视频 ID: {}",
            url
        )))
    }

    /// 通过多个渠道尝试获取视频详情
    async fn fetch_video_from_page(&self, aweme_id: &str) -> Result<DouyinVideoData, DouyinError> {
        // [新增] 渠道0: 尝试本地 Python Sidecar API (dy-mcp)
        // 注意：这里我们需要原始链接，但 fetch_video_from_page 只接收 aweme_id。
        // 为了方便，我们这里构造一个标准的视频链接传给 Sidecar
        eprintln!("[DEBUG] === 尝试渠道0: Local Sidecar API ===");
        let mock_link = format!("https://www.douyin.com/video/{}", aweme_id);
        if let Ok(data) = self.try_local_sidecar_api(&mock_link).await {
            return Ok(data);
        }

        // 渠道1: 尝试新版 detail API
        eprintln!("[DEBUG] === 尝试渠道1: detail API ===");
        let api_url = format!(
            "https://www.douyin.com/aweme/v1/web/aweme/detail/?aweme_id={}&aid=6383",
            aweme_id
        );

        if let Ok(data) = self.try_json_api(&api_url, "aweme_detail", aweme_id).await {
            return Ok(data);
        }

        // 渠道2: 尝试 share 页面获取 meta 信息
        eprintln!("[DEBUG] === 尝试渠道2: Share 页面 ===");
        let share_url = format!("https://www.douyin.com/share/video/{}", aweme_id);

        if let Ok(data) = self.try_share_page(&share_url, aweme_id).await {
            return Ok(data);
        }

        // 渠道3: 尝试旧版 iteminfo API
        eprintln!("[DEBUG] === 尝试渠道3: iteminfo API ===");
        let iteminfo_url = format!(
            "https://www.iesdouyin.com/web/api/v2/aweme/iteminfo/?item_ids={}",
            aweme_id
        );

        if let Ok(data) = self
            .try_json_api(&iteminfo_url, "item_list", aweme_id)
            .await
        {
            return Ok(data);
        }

        Err(DouyinError::ParseError(
            "所有 API 渠道均失败，抖音可能更新了接口。建议使用外部 MCP 服务。".to_string(),
        ))
    }

    /// 尝试本地 Python Sidecar API
    async fn try_local_sidecar_api(&self, link: &str) -> Result<DouyinVideoData, DouyinError> {
        use crate::core::mcp_client::SIDECAR_BASE_URL;
        
        let api_url = format!(
            "{}/parse?link={}",
            SIDECAR_BASE_URL,
            urlencoding::encode(link)
        );

        eprintln!("[DEBUG] 请求 Sidecar: {}", api_url);

        let response = self
            .client
            .get(&api_url)
            .timeout(std::time::Duration::from_secs(45)) // Playwright 可能比较慢
            .send()
            .await
            .map_err(|e| {
                eprintln!("[DEBUG] Sidecar 连接失败: {}", e);
                DouyinError::NetworkError(e.to_string())
            })?;

        let body = response
            .text()
            .await
            .map_err(|e| DouyinError::ParseError(e.to_string()))?;
        eprintln!("[DEBUG] Sidecar 响应长度: {}", body.len());

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| DouyinError::ParseError(format!("JSON 解析失败: {}", e)))?;

        if json["status"].as_str() != Some("success") {
            let error = json["error"].as_str().unwrap_or("未知错误");
            eprintln!("[DEBUG] Sidecar 返回错误: {}", error);
            return Err(DouyinError::ParseError(error.to_string()));
        }

        let data = &json["data"];

        let title = data["title"].as_str().unwrap_or("").to_string();
        let video_url = data["download_url"].as_str().unwrap_or("").to_string();
        let cover_url = data["cover"].as_str().unwrap_or("").to_string();
        let author = data["author"].as_str().unwrap_or("未知").to_string();
        let aweme_id = data["video_id"].as_str().unwrap_or("").to_string();

        eprintln!("[DEBUG] Sidecar 解析成功: {}", title);

        Ok(DouyinVideoData {
            aweme_id,
            title,
            author,
            video_url: video_url.clone(),
            no_watermark_url: video_url, // API 返回的通常已经是无水印的
            author_id: String::new(),
            likes: data["likes"].as_u64().unwrap_or(0),
            comments: data["comments"].as_u64().unwrap_or(0),
            shares: data["shares"].as_u64().unwrap_or(0),
            cover_url,
            duration: 0,
            create_time: 0,
        })
    }

    /// 尝试 JSON API
    async fn try_json_api(
        &self,
        url: &str,
        root_key: &str,
        aweme_id: &str,
    ) -> Result<DouyinVideoData, DouyinError> {
        eprintln!("[DEBUG] 请求: {}", url);

        let response = self
            .client
            .get(url)
            .header("Accept", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Referer", "https://www.douyin.com/")
            .header("Cookie", "ttwid=1%7C1234567890")
            .send()
            .await
            .map_err(|e| DouyinError::NetworkError(e.to_string()))?;

        let body = response
            .text()
            .await
            .map_err(|e| DouyinError::ParseError(e.to_string()))?;
        eprintln!("[DEBUG] 响应长度: {} 字符", body.len());

        if body.is_empty() {
            return Err(DouyinError::ParseError("响应为空".to_string()));
        }

        let preview_len = std::cmp::min(body.len(), 300);
        eprintln!("[DEBUG] 响应预览: {}", &body[..preview_len]);

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| DouyinError::ParseError(format!("JSON 解析失败: {}", e)))?;

        // 根据 root_key 提取数据
        match root_key {
            "aweme_detail" => {
                if let Some(detail) = json.get("aweme_detail") {
                    if !detail.is_null() {
                        return self.extract_video_data_from_api(detail, aweme_id);
                    }
                }
            }
            "item_list" => {
                if let Some(items) = json.get("item_list").and_then(|v| v.as_array()) {
                    if !items.is_empty() {
                        return self.extract_video_data_from_api(&items[0], aweme_id);
                    }
                }
            }
            _ => {}
        }

        Err(DouyinError::ParseError("无有效数据".to_string()))
    }

    /// 尝试从 share 页面获取基本信息
    async fn try_share_page(
        &self,
        url: &str,
        aweme_id: &str,
    ) -> Result<DouyinVideoData, DouyinError> {
        eprintln!("[DEBUG] 请求 Share: {}", url);

        // 使用社交媒体爬虫 UA，通常会返回 meta 标签
        let response = self
            .client
            .get(url)
            .header("User-Agent", "facebookexternalhit/1.1")
            .header("Accept", "text/html")
            .send()
            .await
            .map_err(|e| DouyinError::NetworkError(e.to_string()))?;

        let body = response
            .text()
            .await
            .map_err(|e| DouyinError::ParseError(e.to_string()))?;
        eprintln!("[DEBUG] Share 响应长度: {} 字符", body.len());

        // 提取 meta 标签
        let title = self
            .extract_meta(&body, "og:title")
            .or_else(|| self.extract_meta(&body, "description"))
            .unwrap_or_default();

        let cover = self.extract_meta(&body, "og:image").unwrap_or_default();

        if title.is_empty() {
            return Err(DouyinError::ParseError("Share 页面无有效数据".to_string()));
        }

        eprintln!("[DEBUG] Share 提取成功: {}", title);

        Ok(DouyinVideoData {
            aweme_id: aweme_id.to_string(),
            title,
            author: "未知".to_string(),
            video_url: String::new(),
            no_watermark_url: String::new(),
            author_id: String::new(),
            likes: 0,
            comments: 0,
            shares: 0,
            cover_url: cover,
            duration: 0,
            create_time: 0,
        })
    }

    /// 提取 meta 标签内容
    fn extract_meta(&self, html: &str, property: &str) -> Option<String> {
        // 匹配 <meta property="og:title" content="xxx"> 或 <meta name="description" content="xxx">
        let patterns = [
            format!(
                r#"<meta[^>]+property=["']{}["'][^>]+content=["']([^"']+)["']"#,
                regex::escape(property)
            ),
            format!(
                r#"<meta[^>]+content=["']([^"']+)["'][^>]+property=["']{}["']"#,
                regex::escape(property)
            ),
            format!(
                r#"<meta[^>]+name=["']{}["'][^>]+content=["']([^"']+)["']"#,
                regex::escape(property)
            ),
        ];

        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(html) {
                    if let Some(m) = caps.get(1) {
                        return Some(m.as_str().to_string());
                    }
                }
            }
        }
        None
    }

    /// 从 API 响应中提取视频数据
    fn extract_video_data_from_api(
        &self,
        item: &serde_json::Value,
        aweme_id: &str,
    ) -> Result<DouyinVideoData, DouyinError> {
        // 标题
        let title = item["desc"].as_str().unwrap_or("").to_string();

        // 作者
        let author = item["author"]["nickname"]
            .as_str()
            .unwrap_or("未知")
            .to_string();

        let author_id = item["author"]["unique_id"]
            .as_str()
            .or_else(|| item["author"]["short_id"].as_str())
            .unwrap_or("")
            .to_string();

        // 统计数据
        let statistics = &item["statistics"];
        let likes = statistics["digg_count"].as_u64().unwrap_or(0);
        let comments = statistics["comment_count"].as_u64().unwrap_or(0);
        let shares = statistics["share_count"].as_u64().unwrap_or(0);

        // 视频信息
        let video = &item["video"];
        let duration = video["duration"].as_u64().map(|d| d / 1000).unwrap_or(0);

        // 封面
        let cover_url = video["cover"]["url_list"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 视频地址
        let video_url = video["play_addr"]["url_list"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 无水印地址（替换 playwm 为 play）
        let no_watermark_url = if video_url.contains("playwm") {
            video_url.replace("playwm", "play")
        } else {
            video_url.clone()
        };

        // 创建时间
        let create_time = item["create_time"].as_u64().unwrap_or(0);

        eprintln!("[DEBUG] 解析成功: {} - {}", title, author);

        Ok(DouyinVideoData {
            aweme_id: aweme_id.to_string(),
            video_url,
            no_watermark_url,
            title,
            author,
            author_id,
            likes,
            comments,
            shares,
            cover_url,
            duration,
            create_time,
        })
    }

    /// 从页面 HTML 中解析 RENDER_DATA
    fn parse_render_data(
        &self,
        html: &str,
        aweme_id: &str,
    ) -> Result<DouyinVideoData, DouyinError> {
        // 检查是否是验证页面
        if html.contains("验证码") || html.contains("captcha") || html.contains("verify") {
            return Err(DouyinError::AuthRequired);
        }

        // 尝试多种数据提取模式
        let render_data = self.try_extract_render_data(html)?;

        if render_data.is_empty() {
            // 输出调试信息（前500字符）
            let preview = if html.len() > 500 { &html[..500] } else { html };
            eprintln!("[DEBUG] 页面内容预览: {}", preview);
            return Err(DouyinError::ParseError("页面数据为空".to_string()));
        }

        // 解析 JSON
        let json: serde_json::Value = serde_json::from_str(&render_data)
            .map_err(|e| DouyinError::ParseError(format!("JSON 解析失败: {}", e)))?;

        // 尝试多种路径提取视频数据
        let aweme_detail = self.find_aweme_detail(&json, aweme_id)?;

        self.extract_video_data(&aweme_detail, aweme_id)
    }

    /// 尝试多种模式提取页面数据
    fn try_extract_render_data(&self, html: &str) -> Result<String, DouyinError> {
        // 模式1: RENDER_DATA (最常见)
        let render_data_re =
            Regex::new(r#"<script id="RENDER_DATA" type="application/json">([^<]+)</script>"#)
                .unwrap();
        if let Some(caps) = render_data_re.captures(html) {
            let encoded = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if let Ok(decoded) = urlencoding::decode(encoded) {
                let data = decoded.into_owned();
                if !data.is_empty() {
                    eprintln!("[DEBUG] 使用 RENDER_DATA 提取成功");
                    return Ok(data);
                }
            }
        }

        // 模式2: _ROUTER_DATA
        let router_re = Regex::new(r#"window\._ROUTER_DATA\s*=\s*(\{.+?\});\s*</script>"#).unwrap();
        if let Some(caps) = router_re.captures(html) {
            if let Some(data) = caps.get(1).map(|m| m.as_str().to_string()) {
                if !data.is_empty() {
                    eprintln!("[DEBUG] 使用 _ROUTER_DATA 提取成功");
                    return Ok(data);
                }
            }
        }

        // 模式3: SSR_DATA (新版本可能使用)
        let ssr_re =
            Regex::new(r#"<script id="SSR_DATA" type="application/json">([^<]+)</script>"#)
                .unwrap();
        if let Some(caps) = ssr_re.captures(html) {
            let encoded = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if let Ok(decoded) = urlencoding::decode(encoded) {
                let data = decoded.into_owned();
                if !data.is_empty() {
                    eprintln!("[DEBUG] 使用 SSR_DATA 提取成功");
                    return Ok(data);
                }
            }
        }

        // 模式4: __INITIAL_STATE__
        let initial_state_re = Regex::new(r#"window\.__INITIAL_STATE__\s*=\s*(\{.+?\});"#).unwrap();
        if let Some(caps) = initial_state_re.captures(html) {
            if let Some(data) = caps.get(1).map(|m| m.as_str().to_string()) {
                if !data.is_empty() {
                    eprintln!("[DEBUG] 使用 __INITIAL_STATE__ 提取成功");
                    return Ok(data);
                }
            }
        }

        // 模式5: 直接搜索 aweme_detail JSON
        let aweme_re = Regex::new(r#""aweme_detail"\s*:\s*(\{[^}]+\})"#).unwrap();
        if let Some(caps) = aweme_re.captures(html) {
            if let Some(data) = caps
                .get(1)
                .map(|m| format!(r#"{{"aweme_detail":{}}}"#, m.as_str()))
            {
                eprintln!("[DEBUG] 使用直接 aweme_detail 提取");
                return Ok(data);
            }
        }

        eprintln!("[DEBUG] 所有数据提取模式均失败");
        Err(DouyinError::ParseError("无法找到页面数据".to_string()))
    }

    /// 在 JSON 中查找 aweme_detail
    fn find_aweme_detail(
        &self,
        json: &serde_json::Value,
        aweme_id: &str,
    ) -> Result<serde_json::Value, DouyinError> {
        // 路径1: 直接在根对象中查找
        if let Some(detail) = json.get("aweme_detail") {
            if !detail.is_null() {
                return Ok(detail.clone());
            }
        }

        // 路径2: 在 loaderData 中查找
        if let Some(loader_data) = json.get("loaderData") {
            // 遍历所有 key 查找包含 video 的
            if let Some(obj) = loader_data.as_object() {
                for (key, value) in obj {
                    if key.contains("video") || key.contains("detail") {
                        if let Some(detail) = value.get("aweme_detail") {
                            if !detail.is_null() {
                                return Ok(detail.clone());
                            }
                        }
                        // 尝试直接在 value 中查找
                        if let Some(aweme) = value.get("aweme") {
                            if !aweme.is_null() {
                                return Ok(aweme.clone());
                            }
                        }
                    }
                }
            }
        }

        // 路径3: 递归搜索 aweme_detail
        if let Some(detail) = self.recursive_find_key(json, "aweme_detail") {
            return Ok(detail);
        }

        // 路径4: 搜索匹配 aweme_id 的对象
        if let Some(detail) = self.find_by_aweme_id(json, aweme_id) {
            return Ok(detail);
        }

        Err(DouyinError::VideoNotFound)
    }

    /// 递归查找指定 key
    fn recursive_find_key(&self, json: &serde_json::Value, key: &str) -> Option<serde_json::Value> {
        match json {
            serde_json::Value::Object(map) => {
                if let Some(value) = map.get(key) {
                    if !value.is_null() {
                        return Some(value.clone());
                    }
                }
                for (_, v) in map {
                    if let Some(found) = self.recursive_find_key(v, key) {
                        return Some(found);
                    }
                }
                None
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(found) = self.recursive_find_key(item, key) {
                        return Some(found);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// 通过 aweme_id 查找对应的视频数据
    fn find_by_aweme_id(
        &self,
        json: &serde_json::Value,
        aweme_id: &str,
    ) -> Option<serde_json::Value> {
        match json {
            serde_json::Value::Object(map) => {
                // 检查当前对象是否包含匹配的 aweme_id
                if let Some(id) = map.get("aweme_id").or_else(|| map.get("awemeId")) {
                    if id.as_str() == Some(aweme_id) {
                        return Some(json.clone());
                    }
                }
                // 递归搜索
                for (_, v) in map {
                    if let Some(found) = self.find_by_aweme_id(v, aweme_id) {
                        return Some(found);
                    }
                }
                None
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(found) = self.find_by_aweme_id(item, aweme_id) {
                        return Some(found);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// 从 aweme_detail 中提取视频数据
    fn extract_video_data(
        &self,
        detail: &serde_json::Value,
        aweme_id: &str,
    ) -> Result<DouyinVideoData, DouyinError> {
        // 提取标题
        let title = detail["desc"]
            .as_str()
            .or_else(|| detail["title"].as_str())
            .unwrap_or("")
            .to_string();

        // 作者信息
        let author = detail["author"]["nickname"]
            .as_str()
            .or_else(|| detail["authorInfo"]["nickname"].as_str())
            .unwrap_or("未知")
            .to_string();

        let author_id = detail["author"]["unique_id"]
            .as_str()
            .or_else(|| detail["author"]["short_id"].as_str())
            .or_else(|| detail["authorInfo"]["uniqueId"].as_str())
            .unwrap_or("")
            .to_string();

        // 统计数据
        let statistics = &detail["statistics"];
        let stats = if statistics.is_null() {
            &detail["stats"]
        } else {
            statistics
        };

        let likes = stats["digg_count"]
            .as_u64()
            .or_else(|| stats["diggCount"].as_u64())
            .unwrap_or(0);
        let comments = stats["comment_count"]
            .as_u64()
            .or_else(|| stats["commentCount"].as_u64())
            .unwrap_or(0);
        let shares = stats["share_count"]
            .as_u64()
            .or_else(|| stats["shareCount"].as_u64())
            .unwrap_or(0);

        // 视频信息
        let video = &detail["video"];
        let duration = video["duration"]
            .as_u64()
            .map(|d| d / 1000) // 毫秒转秒
            .unwrap_or(0);

        // 封面
        let cover_url = video["cover"]["url_list"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .or_else(|| {
                video["coverUrlList"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str())
            })
            .or_else(|| video["cover"].as_str())
            .unwrap_or("")
            .to_string();

        // 视频地址 - 尝试获取无水印版本
        let video_url = self.extract_video_url(video);
        let no_watermark_url = self
            .extract_no_watermark_url(video)
            .unwrap_or_else(|| video_url.clone());

        // 创建时间
        let create_time = detail["create_time"]
            .as_u64()
            .or_else(|| detail["createTime"].as_u64())
            .unwrap_or(0);

        Ok(DouyinVideoData {
            aweme_id: aweme_id.to_string(),
            video_url,
            no_watermark_url,
            title,
            author,
            author_id,
            likes,
            comments,
            shares,
            cover_url,
            duration,
            create_time,
        })
    }

    /// 提取视频 URL
    fn extract_video_url(&self, video: &serde_json::Value) -> String {
        // 尝试多种路径
        let paths = [
            &video["play_addr"]["url_list"],
            &video["playAddr"]["urlList"],
            &video["playApi"],
        ];

        for path in paths {
            if let Some(arr) = path.as_array() {
                if let Some(url) = arr.first().and_then(|v| v.as_str()) {
                    return url.to_string();
                }
            }
            if let Some(url) = path.as_str() {
                return url.to_string();
            }
        }

        String::new()
    }

    /// 提取无水印视频 URL
    fn extract_no_watermark_url(&self, video: &serde_json::Value) -> Option<String> {
        // 尝试 H265 无水印版本
        let h265_paths = [
            &video["play_addr_265"]["url_list"],
            &video["playAddr265"]["urlList"],
            &video["bit_rate"],
        ];

        for path in h265_paths {
            if let Some(arr) = path.as_array() {
                // 对于 bit_rate，需要找到最高质量的
                if let Some(first) = arr.first() {
                    if let Some(play_addr) = first.get("play_addr") {
                        if let Some(urls) = play_addr["url_list"].as_array() {
                            if let Some(url) = urls.first().and_then(|v| v.as_str()) {
                                return Some(url.to_string());
                            }
                        }
                    }
                    if let Some(url) = first.as_str() {
                        return Some(url.to_string());
                    }
                }
            }
        }

        // 尝试替换 playwm 为 play
        let video_url = self.extract_video_url(video);
        if video_url.contains("playwm") {
            return Some(video_url.replace("playwm", "play"));
        }

        None
    }

    /// 验证是否为有效的抖音链接
    pub fn is_valid_link(link: &str) -> bool {
        let link = link.trim().to_lowercase();
        link.contains("douyin.com")
            || link.contains("iesdouyin.com")
            || link.contains("v.douyin.com")
    }
}

impl Default for DouyinParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_link() {
        assert!(DouyinParser::is_valid_link("https://v.douyin.com/abc123/"));
        assert!(DouyinParser::is_valid_link(
            "https://www.douyin.com/video/123456789"
        ));
        assert!(!DouyinParser::is_valid_link(
            "https://www.tiktok.com/video/123"
        ));
        assert!(!DouyinParser::is_valid_link("not a url"));
    }

    #[test]
    fn test_extract_aweme_id() {
        let parser = DouyinParser::new();

        let url = "https://www.douyin.com/video/7123456789012345678";
        assert_eq!(parser.extract_aweme_id(url).unwrap(), "7123456789012345678");

        let url = "https://www.douyin.com/note/7123456789012345678";
        assert_eq!(parser.extract_aweme_id(url).unwrap(), "7123456789012345678");
    }
}
