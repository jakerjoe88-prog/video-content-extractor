// MCP 客户端
// 用于调用抖音数据分析服务（dy-mcp 和 Undoom MCP）

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// 本地 Sidecar API 端口
pub const SIDECAR_PORT: u16 = 38190;
/// 本地 Sidecar API 基础 URL
pub const SIDECAR_BASE_URL: &str = "http://127.0.0.1:38190";

#[derive(Error, Debug, Clone)]
pub enum McpError {
    #[error("网络请求失败: {0}")]
    NetworkError(String),
    #[error("链接解析失败: {0}")]
    ParseError(String),
    #[error("服务不可用: {0}")]
    ServiceUnavailable(String),
    #[error("频率限制")]
    RateLimited,
    #[error("链接无效: {0}")]
    InvalidLink(String),
    #[error("请求超时")]
    Timeout,
    #[error("响应解析失败: {0}")]
    ResponseParseError(String),
}

/// MCP 客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub dy_mcp_url: String,
    pub undoom_mcp_url: String,
    pub request_interval_ms: u64,
    pub max_retries: u32,
    pub timeout_secs: u64,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            dy_mcp_url: "http://localhost:3000".to_string(),
            undoom_mcp_url: "http://localhost:3001".to_string(),
            request_interval_ms: 1000,
            max_retries: 3,
            timeout_secs: 30,
        }
    }
}

/// 抖音视频信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DouyinVideoInfo {
    pub video_url: String,
    pub title: String,
    pub author: String,
    pub likes: u64,
    pub comments: u64,
    pub shares: u64,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default, alias = "create_time", alias = "publish_time")]
    pub published_at: Option<String>,
    #[serde(default, alias = "collect_count", alias = "收藏数")]
    pub favorites: Option<u64>,
    #[serde(default)]
    pub topics: Option<Vec<String>>,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub videos: Vec<DouyinVideoInfo>,
    pub total: u64,
}

/// 批量处理进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgress {
    pub current: usize,
    pub total: usize,
    pub success: usize,
    pub failed: usize,
}

/// 单个链接解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkParseResult {
    pub link: String,
    pub success: bool,
    pub video_info: Option<DouyinVideoInfo>,
    pub error: Option<String>,
    pub retry_count: u32,
}

/// dy-mcp 服务响应格式
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
struct DyMcpResponse {
    status: String,
    data: Option<DyMcpVideoData>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
struct DyMcpVideoData {
    #[serde(default)]
    video_url: Option<String>,
    #[serde(default, alias = "no_watermark_url")]
    no_watermark_video_url: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default, alias = "nickname")]
    author: Option<String>,
    #[serde(default, alias = "digg_count")]
    likes: Option<u64>,
    #[serde(default, alias = "comment_count")]
    comments: Option<u64>,
    #[serde(default, alias = "share_count")]
    shares: Option<u64>,
    #[serde(default)]
    cover_url: Option<String>,
    #[serde(default)]
    duration: Option<u64>,
    #[serde(default, alias = "create_time", alias = "publish_time")]
    published_at: Option<serde_json::Value>,
    #[serde(default, alias = "collect_count")]
    favorites: Option<u64>,
    #[serde(default)]
    topics: Option<Vec<String>>,
}

/// Undoom MCP 服务响应格式
#[derive(Debug, Deserialize)]
struct UndoomMcpResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    videos: Option<Vec<UndoomVideoData>>,
    #[serde(default)]
    total: Option<u64>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UndoomVideoData {
    #[serde(default)]
    video_url: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    likes: Option<u64>,
    #[serde(default)]
    comments: Option<u64>,
    #[serde(default)]
    shares: Option<u64>,
    #[serde(default)]
    cover_url: Option<String>,
    #[serde(default)]
    duration: Option<u64>,
    #[serde(default, alias = "create_time", alias = "publish_time")]
    published_at: Option<serde_json::Value>,
    #[serde(default, alias = "collect_count")]
    favorites: Option<u64>,
    #[serde(default)]
    topics: Option<Vec<String>>,
}

fn published_at_string(value: Option<serde_json::Value>) -> Option<String> {
    value.map(|value| value.as_str().map(ToOwned::to_owned).unwrap_or_else(|| value.to_string()))
}

/// MCP 客户端
pub struct McpClient {
    config: McpConfig,
    http_client: Client,
}

impl McpClient {
    /// 创建新的 MCP 客户端实例
    pub fn new(config: McpConfig) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            config,
            http_client,
        }
    }

    /// 获取配置
    pub fn config(&self) -> &McpConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: McpConfig) {
        self.config = config;
    }

    /// 解析抖音链接。
    ///
    /// Python Sidecar contains the browser fallback required by current Douyin
    /// pages, so prefer it over the lightweight Rust parser.  A real browser
    /// fallback can take longer than 30 seconds on a cold or slow connection.
    pub async fn parse_douyin_link(&self, link: &str) -> Result<DouyinVideoInfo, McpError> {
        // 验证链接格式
        if !Self::is_valid_douyin_link(link) {
            return Err(McpError::InvalidLink(link.to_string()));
        }

        match self.parse_douyin_link_via_sidecar(link).await {
            Ok(data) => Ok(data),
            Err(sidecar_error) => {
                let parser = super::douyin_parser::DouyinParser::new();
                parser.parse_link(link).await.map(|data| DouyinVideoInfo {
                video_url: data.no_watermark_url.clone(),
                title: data.title,
                author: data.author,
                likes: data.likes,
                comments: data.comments,
                shares: data.shares,
                cover_url: Some(data.cover_url),
                duration: Some(data.duration),
                published_at: Some(data.create_time.to_string()),
                favorites: None,
                topics: None,
                }).map_err(|rust_error| McpError::ParseError(format!(
                    "链接解析失败（本机解析: {}；备用解析: {}）",
                    sidecar_error, rust_error
                )))
            }
        }
    }

    async fn parse_douyin_link_via_sidecar(&self, link: &str) -> Result<DouyinVideoInfo, McpError> {
        let token = crate::core::sidecar_manager::sidecar_token()
            .ok_or_else(|| McpError::ServiceUnavailable("内置解析服务尚未初始化".to_string()))?;
        // Do not let a desktop proxy intercept loopback traffic.  The Sidecar
        // may need Playwright as a fallback, which routinely exceeds 30s.
        let local_client = Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|error| McpError::NetworkError(format!("创建本机解析客户端失败: {}", error)))?;
        let response = local_client
            .post(format!("{}/parse", SIDECAR_BASE_URL))
            .header("X-Sidecar-Token", token)
            .json(&serde_json::json!({ "link": link }))
            .timeout(Duration::from_secs(120))
            .send().await
            .map_err(|error| McpError::NetworkError(format!("连接兼容解析服务失败: {}", error)))?;
        if !response.status().is_success() {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            return Err(McpError::NetworkError(format!("兼容解析服务返回 {}: {}", status, detail)));
        }
        let payload: serde_json::Value = response.json().await
            .map_err(|error| McpError::ResponseParseError(error.to_string()))?;
        let data = payload.get("data")
            .ok_or_else(|| McpError::ResponseParseError("兼容解析结果缺少 data".to_string()))?;
        let video_url = data.get("download_url").and_then(|value| value.as_str())
            .ok_or_else(|| McpError::ResponseParseError("兼容解析结果缺少视频地址".to_string()))?;
        Ok(DouyinVideoInfo {
            video_url: video_url.to_string(),
            title: data.get("title").and_then(|value| value.as_str()).unwrap_or("抖音作品").to_string(),
            author: data.get("author").and_then(|value| value.as_str()).unwrap_or("未知作者").to_string(),
            likes: data.get("likes").and_then(|value| value.as_u64()).unwrap_or(0),
            comments: data.get("comments").and_then(|value| value.as_u64()).unwrap_or(0),
            shares: data.get("shares").and_then(|value| value.as_u64()).unwrap_or(0),
            cover_url: data.get("cover").and_then(|value| value.as_str()).map(str::to_string),
            duration: None,
            published_at: None,
            favorites: None,
            topics: None,
        })
    }

    /// 带重试的解析
    pub async fn parse_with_retry(&self, link: &str) -> LinkParseResult {
        let mut last_error = String::from("未知错误");
        let mut retry_count = 0;

        for attempt in 0..self.config.max_retries {
            retry_count = attempt;

            match self.parse_douyin_link(link).await {
                Ok(info) => {
                    return LinkParseResult {
                        link: link.to_string(),
                        success: true,
                        video_info: Some(info),
                        error: None,
                        retry_count,
                    };
                }
                Err(e) => {
                    last_error = e.to_string();

                    // 对于某些错误不需要重试
                    match &e {
                        McpError::InvalidLink(_) => break,
                        McpError::RateLimited => {
                            // 频率限制时等待更长时间
                            tokio::time::sleep(Duration::from_millis(
                                self.config.request_interval_ms * 2,
                            ))
                            .await;
                        }
                        _ => {
                            // 其他错误等待标准间隔后重试
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    }
                }
            }
        }

        LinkParseResult {
            link: link.to_string(),
            success: false,
            video_info: None,
            error: Some(last_error),
            retry_count,
        }
    }

    /// 批量解析链接（带重试和进度回调）
    pub async fn parse_links_batch<F>(
        &self,
        links: Vec<String>,
        progress_callback: F,
    ) -> Vec<LinkParseResult>
    where
        F: Fn(BatchProgress),
    {
        let total = links.len();
        let mut results = Vec::with_capacity(total);
        let mut success = 0;
        let mut failed = 0;

        for (i, link) in links.into_iter().enumerate() {
            let result = self.parse_with_retry(&link).await;

            if result.success {
                success += 1;
            } else {
                failed += 1;
            }

            results.push(result);

            progress_callback(BatchProgress {
                current: i + 1,
                total,
                success,
                failed,
            });

            // 请求间隔（最后一个不需要等待）
            if i < total - 1 {
                tokio::time::sleep(Duration::from_millis(self.config.request_interval_ms)).await;
            }
        }

        results
    }

    /// 搜索关键词视频（调用 Undoom MCP 服务）
    pub async fn search_videos(
        &self,
        keyword: &str,
        count: u32,
        scroll_times: u32,
    ) -> Result<SearchResult, McpError> {
        let url = format!("{}/search", self.config.undoom_mcp_url);

        let response = self
            .http_client
            .post(&url)
            .json(&serde_json::json!({
                "keyword": keyword,
                "count": count,
                "scroll_times": scroll_times
            }))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    McpError::Timeout
                } else if e.is_connect() {
                    McpError::ServiceUnavailable(format!("无法连接到 Undoom MCP 服务: {}", e))
                } else {
                    McpError::NetworkError(e.to_string())
                }
            })?;

        let status = response.status();
        if status.as_u16() == 429 {
            return Err(McpError::RateLimited);
        }
        if !status.is_success() {
            return Err(McpError::NetworkError(format!("HTTP 错误: {}", status)));
        }

        let mcp_response: UndoomMcpResponse = response
            .json()
            .await
            .map_err(|e| McpError::ResponseParseError(e.to_string()))?;

        if !mcp_response.success {
            return Err(McpError::ParseError(
                mcp_response.error.unwrap_or_else(|| "搜索失败".to_string()),
            ));
        }

        let videos = mcp_response
            .videos
            .unwrap_or_default()
            .into_iter()
            .map(|v| DouyinVideoInfo {
                video_url: v.video_url.unwrap_or_default(),
                title: v.title.unwrap_or_else(|| "未知标题".to_string()),
                author: v.author.unwrap_or_else(|| "未知作者".to_string()),
                likes: v.likes.unwrap_or(0),
                comments: v.comments.unwrap_or(0),
                shares: v.shares.unwrap_or(0),
                cover_url: v.cover_url,
                duration: v.duration,
                published_at: published_at_string(v.published_at),
                favorites: v.favorites,
                topics: v.topics,
            })
            .collect();

        Ok(SearchResult {
            videos,
            total: mcp_response.total.unwrap_or(0),
        })
    }

    /// 获取用户视频列表（调用 Undoom MCP 服务）
    pub async fn resolve_creator_id(&self, input: &str) -> Result<String, McpError> {
        fn extract(value: &str) -> Option<String> {
            let sec_uid = regex::Regex::new(r"(?i)(?:sec_uid|sec_user_id)=([^&\s]+)")
                .ok()?
                .captures(value)
                .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()));
            if sec_uid.is_some() { return sec_uid; }
            regex::Regex::new(r"(?i)douyin\.com/user/([^/?#\s]+)")
                .ok()?
                .captures(value)
                .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        }

        let input = input.trim();
        if let Some(id) = extract(input) { return Ok(id); }
        if !input.contains("douyin.com") {
            return Err(McpError::InvalidLink("请输入公开的抖音博主主页链接".to_string()));
        }

        let response = self.http_client.get(input)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120 Safari/537.36")
            .send().await.map_err(|e| if e.is_timeout() { McpError::Timeout } else { McpError::NetworkError(e.to_string()) })?;
        let final_url = response.url().to_string();
        extract(&final_url).ok_or_else(|| McpError::ParseError("未能从主页链接识别博主身份，可能需要验证或页面已不可公开访问".to_string()))
    }

    /// 获取用户视频列表（调用 Undoom MCP 服务）
    async fn fetch_public_creator_videos(&self, user_id: &str) -> Result<Vec<DouyinVideoInfo>, McpError> {
        let page_url = format!("https://www.douyin.com/user/{}", user_id);
        let response = self
            .http_client
            .get(&page_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120 Safari/537.36")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
            .map_err(|e| if e.is_timeout() { McpError::Timeout } else { McpError::NetworkError(e.to_string()) })?;
        if !response.status().is_success() {
            return Err(McpError::NetworkError(format!("主页返回 HTTP {}", response.status())));
        }
        let html = response.text().await.map_err(|e| McpError::ResponseParseError(e.to_string()))?;
        if html.contains("验证") || html.to_ascii_lowercase().contains("captcha") {
            return Err(McpError::ServiceUnavailable("抖音主页触发验证码或风控验证，请在浏览器完成验证后重试".to_string()));
        }
        let mut values = Vec::new();

        for pattern in [
            r#"(?s)<script[^>]+id=[\"']RENDER_DATA[\"'][^>]*>(.*?)</script>"#,
            r#"(?s)window\._ROUTER_DATA\s*=\s*(\{.+?\})\s*;\s*</script>"#,
            r#"(?s)<script[^>]*>(\{\s*[\"']?aweme_list[\"']?.+?\})</script>"#,
        ] {
            let regex = regex::Regex::new(pattern).map_err(|e| McpError::ResponseParseError(e.to_string()))?;
            for captures in regex.captures_iter(&html) {
                if let Some(raw) = captures.get(1) {
                    let candidate = urlencoding::decode(raw.as_str())
                        .map(|value| value.into_owned())
                        .unwrap_or_else(|_| raw.as_str().to_string());
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&candidate) {
                        values.push(value);
                    }
                }
            }
        }

        let mut objects = Vec::new();
        for value in values {
            Self::collect_creator_items(&value, &mut objects);
        }

        let mut videos = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for item in objects {
            let aweme_id = item.get("aweme_id").and_then(Self::json_string).unwrap_or_default();
            if aweme_id.is_empty() || !seen.insert(aweme_id.clone()) { continue; }
            let title = item.get("desc").and_then(Self::json_string).unwrap_or_else(|| "未命名作品".to_string());
            let author = item.get("author").and_then(|v| v.get("nickname")).and_then(Self::json_string).unwrap_or_else(|| "未知作者".to_string());
            let statistics = item.get("statistics");
            let likes = statistics.and_then(|v| v.get("digg_count")).and_then(Self::json_u64).unwrap_or(0);
            let comments = statistics.and_then(|v| v.get("comment_count")).and_then(Self::json_u64).unwrap_or(0);
            let shares = statistics.and_then(|v| v.get("share_count")).and_then(Self::json_u64).unwrap_or(0);
            let favorites = statistics.and_then(|v| v.get("collect_count")).and_then(Self::json_u64);
            let cover_url = item.get("video").and_then(|v| v.get("cover")).and_then(Self::first_url)
                .or_else(|| item.get("video").and_then(|v| v.get("origin_cover")).and_then(Self::first_url));
            let duration = item.get("video").and_then(|v| v.get("duration")).and_then(Self::json_u64).map(|value| value / 1000);
            let published_at = item.get("create_time").and_then(Self::json_string);
            let topics = item.get("text_extra").and_then(|value| value.as_array()).map(|items| items.iter().filter_map(|item| item.get("hashtag_name").and_then(Self::json_string)).collect::<Vec<_>>());
            videos.push(DouyinVideoInfo {
                video_url: format!("https://www.douyin.com/video/{}", aweme_id),
                title,
                author,
                likes,
                comments,
                shares,
                cover_url,
                duration,
                published_at,
                favorites,
                topics,
            });
        }

        if videos.is_empty() { Err(McpError::ParseError("公开主页没有返回可识别的作品数据，可能需要验证或页面结构已变化".to_string())) } else { Ok(videos) }
    }

    fn json_string(value: &serde_json::Value) -> Option<String> {
        value.as_str().map(ToOwned::to_owned).or_else(|| value.as_u64().map(|value| value.to_string()))
    }

    fn json_u64(value: &serde_json::Value) -> Option<u64> {
        value.as_u64().or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    }

    fn first_url(value: &serde_json::Value) -> Option<String> {
        value.get("url_list").and_then(|urls| urls.as_array()).and_then(|urls| urls.iter().find_map(Self::json_string))
    }

    fn collect_creator_items(value: &serde_json::Value, output: &mut Vec<serde_json::Value>) {
        match value {
            serde_json::Value::Object(map) => {
                if map.contains_key("aweme_id") && map.contains_key("statistics") { output.push(value.clone()); }
                for child in map.values() { Self::collect_creator_items(child, output); }
            }
            serde_json::Value::Array(items) => {
                for child in items { Self::collect_creator_items(child, output); }
            }
            _ => {}
        }
    }

    /// 获取用户视频列表（公开主页直取失败时调用 Undoom MCP 服务）
    pub async fn get_user_videos(&self, user_id: &str) -> Result<Vec<DouyinVideoInfo>, McpError> {
        match tokio::time::timeout(Duration::from_secs(20), self.fetch_public_creator_videos(user_id)).await {
            Err(_) => return Err(McpError::Timeout),
            Ok(result) => match result {
                Ok(videos) => return Ok(videos),
                Err(McpError::ServiceUnavailable(message)) => return Err(McpError::ServiceUnavailable(message)),
                Err(_) => {}
            },
        }
        let url = format!("{}/user/videos", self.config.undoom_mcp_url);

        let response = self
            .http_client
            .post(&url)
            .json(&serde_json::json!({ "user_id": user_id }))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    McpError::Timeout
                } else if e.is_connect() {
                    McpError::ServiceUnavailable(format!("无法连接到 Undoom MCP 服务: {}", e))
                } else {
                    McpError::NetworkError(e.to_string())
                }
            })?;

        let status = response.status();
        if status.as_u16() == 429 {
            return Err(McpError::RateLimited);
        }
        if !status.is_success() {
            return Err(McpError::NetworkError(format!("HTTP 错误: {}", status)));
        }

        let mcp_response: UndoomMcpResponse = response
            .json()
            .await
            .map_err(|e| McpError::ResponseParseError(e.to_string()))?;

        if !mcp_response.success {
            return Err(McpError::ParseError(
                mcp_response
                    .error
                    .unwrap_or_else(|| "获取用户视频失败".to_string()),
            ));
        }

        let videos = mcp_response
            .videos
            .unwrap_or_default()
            .into_iter()
            .map(|v| DouyinVideoInfo {
                video_url: v.video_url.unwrap_or_default(),
                title: v.title.unwrap_or_else(|| "未知标题".to_string()),
                author: v.author.unwrap_or_else(|| "未知作者".to_string()),
                likes: v.likes.unwrap_or(0),
                comments: v.comments.unwrap_or(0),
                shares: v.shares.unwrap_or(0),
                cover_url: v.cover_url,
                duration: v.duration,
                published_at: published_at_string(v.published_at),
                favorites: v.favorites,
                topics: v.topics,
            })
            .collect();

        Ok(videos)
    }

    /// 验证抖音链接格式
    pub fn is_valid_douyin_link(link: &str) -> bool {
        let link = link.trim().to_lowercase();
        link.contains("douyin.com")
            || link.contains("v.douyin.com")
            || link.contains("iesdouyin.com")
    }

    /// 检查 dy-mcp 服务是否可用（内置解析器始终可用）
    /// 检查 dy-mcp 服务是否可用（内置解析器始终可用）
    pub async fn check_dy_mcp_health(&self) -> bool {
        // 检查本地 Sidecar API
        let health_url = format!("{}/health/auth", SIDECAR_BASE_URL);
        let Some(token) = crate::core::sidecar_manager::sidecar_token() else {
            return false;
        };
        match self
            .http_client
            .get(&health_url)
            .header("X-Sidecar-Token", token)
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// 通过 Python Sidecar 下载视频 (避免 403 问题)
    pub async fn download_video_via_sidecar(
        &self,
        url: &str,
        save_path: &std::path::Path,
    ) -> Result<(), McpError> {
        let api_url = format!("{}/download", SIDECAR_BASE_URL);
        let sidecar_token = crate::core::sidecar_manager::sidecar_token()
            .ok_or_else(|| McpError::ServiceUnavailable("Python Sidecar 尚未初始化".to_string()))?;

        let path_str = save_path
            .to_str()
            .ok_or_else(|| McpError::NetworkError("无效的文件路径".to_string()))?;

        let response = self
            .http_client
            .post(api_url)
            .header("X-Sidecar-Token", sidecar_token)
            .json(&serde_json::json!({
                "url": url,
                "path": path_str
            }))
            .timeout(Duration::from_secs(600)) // 下载可能需要较长时间
            .send()
            .await
            .map_err(|e| McpError::NetworkError(format!("连接 Python 下载服务失败: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
                return Err(McpError::ServiceUnavailable(
                    "内置下载服务会话异常，请关闭工作台后重新打开；系统会自动接管旧服务".to_string(),
                ));
            }
            return Err(McpError::NetworkError(format!(
                "Python 下载服务返回错误 ({}): {}",
                status, error_text
            )));
        }

        // 检查返回的 JSON 状态
        let json_resp: serde_json::Value = response
            .json()
            .await
            .map_err(|e| McpError::ResponseParseError(e.to_string()))?;

        if let Some(status) = json_resp.get("status") {
            if status.as_str() == Some("success") {
                Ok(())
            } else {
                let error = json_resp
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("未知错误");
                Err(McpError::NetworkError(format!(
                    "Python 下载服务执行失败: {}",
                    error
                )))
            }
        } else {
            Err(McpError::ResponseParseError("响应格式无效".to_string()))
        }
    }

    /// 检查 Undoom MCP 服务是否可用
    pub async fn check_undoom_mcp_health(&self) -> bool {
        let url = format!("{}/health", self.config.undoom_mcp_url);
        self.http_client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new(McpConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::McpClient;

    #[test]
    fn creator_payload_items_are_found_recursively() {
        let payload = serde_json::json!({
            "data": {"aweme_list": [{
                "aweme_id": "123",
                "desc": "测试作品",
                "statistics": {"digg_count": 42}
            }]}
        });
        let mut items = Vec::new();
        McpClient::collect_creator_items(&payload, &mut items);
        assert_eq!(items.len(), 1);
        assert_eq!(McpClient::json_string(items[0].get("aweme_id").unwrap()), Some("123".to_string()));
        assert_eq!(McpClient::json_u64(items[0].get("statistics").unwrap().get("digg_count").unwrap()), Some(42));
    }

    #[test]
    fn creator_payload_collects_all_aweme_objects_before_deduplication() {
        let payload = serde_json::json!({
            "aweme_list": [
                {"aweme_id": "1", "statistics": {}},
                {"aweme_id": "1", "statistics": {}}
            ]
        });
        let mut items = Vec::new();
        McpClient::collect_creator_items(&payload, &mut items);
        assert_eq!(items.len(), 2);
    }
}
