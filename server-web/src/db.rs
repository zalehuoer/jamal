//! 数据库模块 - SQLite 持久化存储

use rusqlite::{Connection, Result, params};
use std::path::PathBuf;
use std::sync::Mutex;
use chrono::{DateTime, Utc};

/// 数据库连接包装
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// 创建数据库连接
    pub fn new(db_path: PathBuf) -> Result<Self> {
        // 确保目录存在
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        let conn = Connection::open(&db_path)?;
        
        // 启用 WAL 模式提高并发性能
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        
        let db = Self { conn: Mutex::new(conn) };
        db.init_tables()?;
        Ok(db)
    }

    /// 初始化数据库表
    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute_batch(r#"
            -- 监听器配置表
            CREATE TABLE IF NOT EXISTS listeners (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                bind_address TEXT NOT NULL,
                port INTEGER NOT NULL,
                encryption_key TEXT NOT NULL,
                is_running INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );

            -- 客户端信息表
            CREATE TABLE IF NOT EXISTS clients (
                id TEXT PRIMARY KEY,
                ip_address TEXT,
                hostname TEXT,
                username TEXT,
                os_version TEXT,
                tag TEXT,
                is_elevated INTEGER DEFAULT 0,
                beacon_interval INTEGER DEFAULT 30,
                listener_id TEXT,
                first_seen TEXT DEFAULT CURRENT_TIMESTAMP,
                last_seen TEXT,
                country TEXT,
                country_code TEXT
            );

            -- Shell 命令历史表
            CREATE TABLE IF NOT EXISTS shell_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                client_id TEXT NOT NULL,
                command TEXT NOT NULL,
                output TEXT,
                success INTEGER DEFAULT 1,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );

            -- 操作日志表
            CREATE TABLE IF NOT EXISTS operation_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                client_id TEXT,
                operation_type TEXT,
                details TEXT,
                success INTEGER DEFAULT 1,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
        "#)?;
        
        Ok(())
    }

    // ============== 监听器操作 ==============

    /// 保存监听器
    pub fn save_listener(&self, listener: &ListenerRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO listeners (id, name, bind_address, port, encryption_key, is_running, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                listener.id,
                listener.name,
                listener.bind_address,
                listener.port,
                listener.encryption_key,
                listener.is_running as i32,
                listener.created_at,
            ],
        )?;
        Ok(())
    }

    /// 获取所有监听器
    pub fn get_all_listeners(&self) -> Result<Vec<ListenerRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, bind_address, port, encryption_key, is_running, created_at FROM listeners")?;
        
        let listeners = stmt.query_map([], |row| {
            Ok(ListenerRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                bind_address: row.get(2)?,
                port: row.get(3)?,
                encryption_key: row.get(4)?,
                is_running: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        
        Ok(listeners)
    }

    /// 删除监听器
    pub fn delete_listener(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM listeners WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 更新监听器运行状态
    pub fn update_listener_status(&self, id: &str, is_running: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE listeners SET is_running = ?1 WHERE id = ?2",
            params![is_running as i32, id],
        )?;
        Ok(())
    }

    // ============== 客户端操作 ==============

    /// 保存或更新客户端
    pub fn save_client(&self, client: &ClientRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO clients 
             (id, ip_address, hostname, username, os_version, tag, is_elevated, beacon_interval, listener_id, first_seen, last_seen, country, country_code) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                client.id,
                client.ip_address,
                client.hostname,
                client.username,
                client.os_version,
                client.tag,
                client.is_elevated as i32,
                client.beacon_interval,
                client.listener_id,
                client.first_seen,
                client.last_seen,
                client.country,
                client.country_code,
            ],
        )?;
        Ok(())
    }

    /// 更新客户端最后在线时间
    pub fn update_client_last_seen(&self, id: &str, last_seen: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clients SET last_seen = ?1 WHERE id = ?2",
            params![last_seen, id],
        )?;
        Ok(())
    }

    /// 获取所有客户端
    pub fn get_all_clients(&self) -> Result<Vec<ClientRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, ip_address, hostname, username, os_version, tag, is_elevated, beacon_interval, listener_id, first_seen, last_seen, country, country_code FROM clients"
        )?;
        
        let clients = stmt.query_map([], |row| {
            Ok(ClientRecord {
                id: row.get(0)?,
                ip_address: row.get(1)?,
                hostname: row.get(2)?,
                username: row.get(3)?,
                os_version: row.get(4)?,
                tag: row.get(5)?,
                is_elevated: row.get::<_, i32>(6)? != 0,
                beacon_interval: row.get(7)?,
                listener_id: row.get(8)?,
                first_seen: row.get(9)?,
                last_seen: row.get(10)?,
                country: row.get(11)?,
                country_code: row.get(12)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        
        Ok(clients)
    }

    /// 删除客户端
    pub fn delete_client(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clients WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ============== Shell 历史操作 ==============

    /// 记录 Shell 命令
    pub fn log_shell_command(&self, client_id: &str, command: &str, output: Option<&str>, success: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO shell_history (client_id, command, output, success) VALUES (?1, ?2, ?3, ?4)",
            params![client_id, command, output, success as i32],
        )?;
        Ok(())
    }

    /// 获取客户端的 Shell 历史
    pub fn get_shell_history(&self, client_id: &str, limit: usize) -> Result<Vec<ShellHistoryRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, client_id, command, output, success, created_at 
             FROM shell_history 
             WHERE client_id = ?1 
             ORDER BY created_at DESC 
             LIMIT ?2"
        )?;
        
        let history = stmt.query_map(params![client_id, limit as i64], |row| {
            Ok(ShellHistoryRecord {
                id: row.get(0)?,
                client_id: row.get(1)?,
                command: row.get(2)?,
                output: row.get(3)?,
                success: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        
        Ok(history)
    }

    // ============== 操作日志 ==============

    /// 记录操作日志
    pub fn log_operation(&self, client_id: Option<&str>, operation_type: &str, details: &str, success: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO operation_logs (client_id, operation_type, details, success) VALUES (?1, ?2, ?3, ?4)",
            params![client_id, operation_type, details, success as i32],
        )?;
        Ok(())
    }
}

// ============== 数据记录结构 ==============

#[derive(Debug, Clone)]
pub struct ListenerRecord {
    pub id: String,
    pub name: String,
    pub bind_address: String,
    pub port: i32,
    pub encryption_key: String,
    pub is_running: bool,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ClientRecord {
    pub id: String,
    pub ip_address: Option<String>,
    pub hostname: Option<String>,
    pub username: Option<String>,
    pub os_version: Option<String>,
    pub tag: Option<String>,
    pub is_elevated: bool,
    pub beacon_interval: i32,
    pub listener_id: Option<String>,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ShellHistoryRecord {
    pub id: i64,
    pub client_id: String,
    pub command: String,
    pub output: Option<String>,
    pub success: bool,
    pub created_at: String,
}
