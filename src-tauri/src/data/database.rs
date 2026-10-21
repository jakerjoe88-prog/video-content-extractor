// SQLite 数据库操作

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("数据库连接失败: {0}")]
    ConnectionFailed(String),
    #[error("查询失败: {0}")]
    QueryFailed(String),
    #[error("数据不存在: {0}")]
    NotFound(String),
    #[error("数据库初始化失败: {0}")]
    InitFailed(String),
    #[error("连接池错误: {0}")]
    PoolError(String),
}

impl From<rusqlite::Error> for DbError {
    fn from(err: rusqlite::Error) -> Self {
        DbError::QueryFailed(err.to_string())
    }
}

impl From<r2d2::Error> for DbError {
    fn from(err: r2d2::Error) -> Self {
        DbError::PoolError(err.to_string())
    }
}

/// 数据库连接池类型
pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConnection = PooledConnection<SqliteConnectionManager>;

/// 数据库管理器
#[derive(Clone)]
pub struct Database {
    pool: Arc<DbPool>,
}

impl Database {
    /// 初始化数据库
    /// 
    /// # Arguments
    /// * `db_path` - 数据库文件路径
    /// 
    /// # Returns
    /// * `Result<Self, DbError>` - 数据库实例或错误
    pub fn init(db_path: &str) -> Result<Self, DbError> {
        // 确保父目录存在
        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DbError::InitFailed(format!("创建目录失败: {}", e)))?;
        }

        // 创建连接管理器
        let manager = SqliteConnectionManager::file(db_path);
        
        // 创建连接池
        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;

        let db = Self {
            pool: Arc::new(pool),
        };

        // 初始化 schema
        db.create_schema()?;

        Ok(db)
    }

    /// 获取数据库连接
    pub fn get_connection(&self) -> Result<DbConnection, DbError> {
        self.pool.get().map_err(|e| DbError::PoolError(e.to_string()))
    }

    /// 创建数据库 schema
    pub fn create_schema(&self) -> Result<(), DbError> {
        let conn = self.get_connection()?;
        
        // 启用外键约束
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        
        // 执行 schema 创建
        conn.execute_batch(SCHEMA_SQL)?;
        
        Ok(())
    }

    /// 获取配置值
    pub fn get_config(&self, key: &str) -> Result<Option<String>, DbError> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT value FROM config WHERE key = ?")?;
        
        let result = stmt.query_row(params![key], |row| row.get::<_, String>(0));
        
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::QueryFailed(e.to_string())),
        }
    }

    /// 设置配置值
    pub fn set_config(&self, key: &str, value: &str) -> Result<(), DbError> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value, updated_at) VALUES (?, ?, datetime('now'))",
            params![key, value],
        )?;
        Ok(())
    }

    /// 删除配置值
    pub fn delete_config(&self, key: &str) -> Result<(), DbError> {
        let conn = self.get_connection()?;
        conn.execute("DELETE FROM config WHERE key = ?", params![key])?;
        Ok(())
    }

    /// 获取所有配置
    pub fn get_all_configs(&self) -> Result<Vec<(String, String)>, DbError> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT key, value FROM config")?;
        
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        
        let mut configs = Vec::new();
        for row in rows {
            configs.push(row?);
        }
        
        Ok(configs)
    }

    /// 添加历史记录
    pub fn add_history(
        &self,
        id: &str,
        history_type: &str,
        source: &str,
        status: &str,
    ) -> Result<(), DbError> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO history (id, type, source, status, created_at) VALUES (?, ?, ?, ?, datetime('now'))",
            params![id, history_type, source, status],
        )?;
        Ok(())
    }

    /// 更新历史记录状态
    pub fn update_history_status(
        &self,
        id: &str,
        status: &str,
        result: Option<&str>,
    ) -> Result<(), DbError> {
        let conn = self.get_connection()?;
        
        if let Some(result_text) = result {
            conn.execute(
                "UPDATE history SET status = ?, result = ?, completed_at = datetime('now') WHERE id = ?",
                params![status, result_text, id],
            )?;
        } else {
            conn.execute(
                "UPDATE history SET status = ? WHERE id = ?",
                params![status, id],
            )?;
        }
        
        Ok(())
    }

    /// 获取历史记录
    pub fn get_history(&self, limit: i32) -> Result<Vec<HistoryRecord>, DbError> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, type, source, result, status, created_at, completed_at 
             FROM history ORDER BY created_at DESC LIMIT ?"
        )?;
        
        let rows = stmt.query_map(params![limit], |row| {
            Ok(HistoryRecord {
                id: row.get(0)?,
                history_type: row.get(1)?,
                source: row.get(2)?,
                result: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                completed_at: row.get(6)?,
            })
        })?;
        
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        
        Ok(records)
    }
}

/// 历史记录结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HistoryRecord {
    pub id: String,
    pub history_type: String,
    pub source: String,
    pub result: Option<String>,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// 数据库 Schema SQL
pub const SCHEMA_SQL: &str = r#"
-- 配置表
CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 处理历史
CREATE TABLE IF NOT EXISTS history (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    source TEXT NOT NULL,
    result TEXT,
    status TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
);

-- 知识库文档
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    content TEXT NOT NULL,
    file_path TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- AI 调用统计
CREATE TABLE IF NOT EXISTS ai_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,
    tokens_used INTEGER,
    cost_estimate REAL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_history_type ON history(type);
CREATE INDEX IF NOT EXISTS idx_history_status ON history(status);
CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at);
CREATE INDEX IF NOT EXISTS idx_documents_category ON documents(category);
CREATE INDEX IF NOT EXISTS idx_ai_usage_provider ON ai_usage(provider);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_init() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::init(db_path.to_str().unwrap()).unwrap();
        
        // 验证可以获取连接
        let conn = db.get_connection();
        assert!(conn.is_ok());
    }

    #[test]
    fn test_config_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::init(db_path.to_str().unwrap()).unwrap();
        
        // 设置配置
        db.set_config("test_key", "test_value").unwrap();
        
        // 获取配置
        let value = db.get_config("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));
        
        // 获取不存在的配置
        let none_value = db.get_config("nonexistent").unwrap();
        assert_eq!(none_value, None);
        
        // 删除配置
        db.delete_config("test_key").unwrap();
        let deleted = db.get_config("test_key").unwrap();
        assert_eq!(deleted, None);
    }

    #[test]
    fn test_history_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::init(db_path.to_str().unwrap()).unwrap();
        
        // 添加历史记录
        db.add_history("test-id-1", "local_video", "/path/to/video.mp4", "processing").unwrap();
        
        // 更新状态
        db.update_history_status("test-id-1", "completed", Some("转写结果")).unwrap();
        
        // 获取历史记录
        let history = db.get_history(10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "test-id-1");
        assert_eq!(history[0].status, "completed");
    }
}
