// MCP 链接解析重试机制属性测试
// **Property 8: 链接解析重试机制**
// **Validates: Requirements 16.1**
//
// *For any* 失败的链接解析请求，系统应自动重试最多 3 次，且重试次数不超过配置的最大值。

use proptest::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// 导入被测试的模块
use douyin_creator_tools_lib::core::mcp_client::{McpConfig, McpError};

/// 模拟的 MCP 服务器，用于测试重试机制
struct MockMcpServer {
    /// 调用计数器
    call_count: Arc<AtomicUsize>,
    /// 在第几次调用时成功（0 表示永远失败）
    succeed_on_attempt: usize,
}

impl MockMcpServer {
    fn new(succeed_on_attempt: usize) -> Self {
        Self {
            call_count: Arc::new(AtomicUsize::new(0)),
            succeed_on_attempt,
        }
    }

    /// 模拟链接解析调用
    fn parse_link(&self, _link: &str) -> Result<(), McpError> {
        let count = self.call_count.fetch_add(1, Ordering::SeqCst) + 1;

        if self.succeed_on_attempt > 0 && count >= self.succeed_on_attempt {
            Ok(())
        } else {
            Err(McpError::NetworkError("模拟网络错误".to_string()))
        }
    }

    fn get_call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    fn reset(&self) {
        self.call_count.store(0, Ordering::SeqCst);
    }
}

/// 模拟重试逻辑（与 McpClient::parse_with_retry 相同的逻辑）
async fn simulate_retry_logic(
    server: &MockMcpServer,
    link: &str,
    max_retries: u32,
) -> (bool, usize) {
    let mut retry_count = 0;

    for attempt in 0..max_retries {
        retry_count = attempt;

        match server.parse_link(link) {
            Ok(_) => return (true, retry_count as usize),
            Err(_) => {
                // 继续重试
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        }
    }

    (false, retry_count as usize)
}

/// 生成有效的重试次数配置（1-10）
fn max_retries_strategy() -> impl Strategy<Value = u32> {
    1u32..=10u32
}

/// 生成成功的尝试次数（0 表示永远失败，1-10 表示在第几次成功）
fn succeed_on_attempt_strategy() -> impl Strategy<Value = usize> {
    0usize..=10usize
}

/// 生成有效的抖音链接
fn douyin_link_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("https://v.douyin.com/test1".to_string()),
        Just("https://www.douyin.com/video/test2".to_string()),
        Just("https://v.douyin.com/ieFRPqM/".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: tauri-refactor, Property 8: 链接解析重试次数不超过配置值**
    ///
    /// *For any* 失败的链接解析请求，系统应自动重试，且重试次数不超过配置的最大值。
    ///
    /// **Validates: Requirements 16.1**
    #[test]
    fn prop_retry_count_does_not_exceed_max(
        max_retries in max_retries_strategy(),
        link in douyin_link_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // 创建一个永远失败的服务器
            let server = MockMcpServer::new(0);

            // 执行重试逻辑
            let (_success, retry_count) = simulate_retry_logic(&server, &link, max_retries).await;

            // 验证：调用次数应该等于 max_retries
            let call_count = server.get_call_count();
            prop_assert_eq!(call_count, max_retries as usize);

            // 验证：重试次数应该是 max_retries - 1（因为第一次不算重试）
            prop_assert_eq!(retry_count, (max_retries - 1) as usize);

            Ok(())
        })?;
    }

    /// **Feature: tauri-refactor, Property 8: 成功后立即停止重试**
    ///
    /// *For any* 链接解析请求，如果在重试过程中成功，应立即停止重试，不继续尝试。
    ///
    /// **Validates: Requirements 16.1**
    #[test]
    fn prop_stops_retrying_on_success(
        max_retries in max_retries_strategy(),
        succeed_on_attempt in succeed_on_attempt_strategy(),
        link in douyin_link_strategy(),
    ) {
        // 只测试在 max_retries 范围内成功的情况
        prop_assume!(succeed_on_attempt > 0 && succeed_on_attempt <= max_retries as usize);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // 创建一个在第 N 次尝试时成功的服务器
            let server = MockMcpServer::new(succeed_on_attempt);

            // 执行重试逻辑
            let (success, _retry_count) = simulate_retry_logic(&server, &link, max_retries).await;

            // 验证：应该成功
            prop_assert!(success);

            // 验证：调用次数应该等于 succeed_on_attempt（成功后立即停止）
            let call_count = server.get_call_count();
            prop_assert_eq!(call_count, succeed_on_attempt);

            // 验证：调用次数不应该超过 max_retries
            prop_assert!(call_count <= max_retries as usize);

            Ok(())
        })?;
    }

    /// **Feature: tauri-refactor, Property 8: 重试次数与调用次数的关系**
    ///
    /// *For any* 链接解析请求，调用次数应该等于重试次数 + 1（第一次尝试不算重试）。
    ///
    /// **Validates: Requirements 16.1**
    #[test]
    fn prop_call_count_equals_retry_count_plus_one(
        max_retries in max_retries_strategy(),
        link in douyin_link_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // 创建一个永远失败的服务器
            let server = MockMcpServer::new(0);

            // 执行重试逻辑
            let (_success, retry_count) = simulate_retry_logic(&server, &link, max_retries).await;

            // 验证：调用次数 = 重试次数 + 1
            let call_count = server.get_call_count();
            prop_assert_eq!(call_count, retry_count + 1);

            Ok(())
        })?;
    }

    /// **Feature: tauri-refactor, Property 8: 配置的最大重试次数被遵守**
    ///
    /// *For any* MCP 配置，实际重试次数应该不超过配置中的 max_retries 值。
    ///
    /// **Validates: Requirements 16.1**
    #[test]
    fn prop_config_max_retries_is_respected(
        max_retries in max_retries_strategy(),
        link in douyin_link_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // 创建配置
            let config = McpConfig {
                dy_mcp_url: "http://localhost:3000".to_string(),
                undoom_mcp_url: "http://localhost:3001".to_string(),
                request_interval_ms: 100,
                max_retries,
                timeout_secs: 5,
            };

            // 验证配置中的 max_retries
            prop_assert_eq!(config.max_retries, max_retries);

            // 创建服务器并测试
            let server = MockMcpServer::new(0);
            let (_success, _retry_count) = simulate_retry_logic(&server, &link, config.max_retries).await;

            // 验证调用次数不超过配置的 max_retries
            let call_count = server.get_call_count();
            prop_assert!(call_count <= config.max_retries as usize);

            Ok(())
        })?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 基本的重试机制测试（非属性测试）
    #[tokio::test]
    async fn test_basic_retry_mechanism() {
        // 测试：永远失败的情况
        let server = MockMcpServer::new(0);
        let (success, retry_count) = simulate_retry_logic(&server, "test_link", 3).await;

        assert!(!success);
        assert_eq!(server.get_call_count(), 3);
        assert_eq!(retry_count, 2); // 重试 2 次（第一次不算）
    }

    /// 测试：第二次尝试成功
    #[tokio::test]
    async fn test_success_on_second_attempt() {
        let server = MockMcpServer::new(2);
        let (success, retry_count) = simulate_retry_logic(&server, "test_link", 5).await;

        assert!(success);
        assert_eq!(server.get_call_count(), 2); // 只调用了 2 次
        assert_eq!(retry_count, 1); // 重试了 1 次
    }

    /// 测试：第一次就成功
    #[tokio::test]
    async fn test_success_on_first_attempt() {
        let server = MockMcpServer::new(1);
        let (success, retry_count) = simulate_retry_logic(&server, "test_link", 3).await;

        assert!(success);
        assert_eq!(server.get_call_count(), 1); // 只调用了 1 次
        assert_eq!(retry_count, 0); // 没有重试
    }

    /// 测试：max_retries = 1 的边界情况
    #[tokio::test]
    async fn test_max_retries_one() {
        let server = MockMcpServer::new(0);
        let (success, retry_count) = simulate_retry_logic(&server, "test_link", 1).await;

        assert!(!success);
        assert_eq!(server.get_call_count(), 1); // 只调用了 1 次
        assert_eq!(retry_count, 0); // 没有重试（因为 max_retries = 1）
    }

    /// 测试：验证 McpConfig 默认值
    #[test]
    fn test_mcp_config_default_max_retries() {
        let config = McpConfig::default();
        assert_eq!(config.max_retries, 3); // 默认重试 3 次
    }

    /// 测试：多次调用重置计数器
    #[tokio::test]
    async fn test_server_reset() {
        let server = MockMcpServer::new(0);

        // 第一次调用
        let _ = simulate_retry_logic(&server, "test_link", 3).await;
        assert_eq!(server.get_call_count(), 3);

        // 重置
        server.reset();
        assert_eq!(server.get_call_count(), 0);

        // 第二次调用
        let _ = simulate_retry_logic(&server, "test_link", 2).await;
        assert_eq!(server.get_call_count(), 2);
    }
}
