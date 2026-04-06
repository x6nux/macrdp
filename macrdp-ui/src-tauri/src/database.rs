use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

/// SQLite database for storing connection history and traffic statistics.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        // Create tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS connections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                client_ip TEXT NOT NULL,
                client_name TEXT NOT NULL DEFAULT '',
                connected_at TEXT NOT NULL,
                disconnected_at TEXT,
                duration_secs INTEGER DEFAULT 0,
                bytes_total INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS traffic_daily (
                date TEXT PRIMARY KEY,
                bytes_sent INTEGER DEFAULT 0,
                connection_count INTEGER DEFAULT 0
            );",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Record a client disconnection event.
    ///
    /// Inserts a row into `connections` and upserts `traffic_daily` for today.
    pub fn record_disconnection(
        &self,
        client_ip: &str,
        client_name: Option<&str>,
        bytes_total: u64,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let now = chrono::Utc::now();
        let disconnected_at = now.to_rfc3339();
        // We don't have the exact connected_at time from the IPC message,
        // so use disconnected_at as a reasonable fallback.
        let connected_at = &disconnected_at;
        let client_name = client_name.unwrap_or("");
        let today = now.format("%Y-%m-%d").to_string();

        conn.execute(
            "INSERT INTO connections (client_ip, client_name, connected_at, disconnected_at, duration_secs, bytes_total)
             VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            rusqlite::params![client_ip, client_name, connected_at, disconnected_at, bytes_total as i64],
        )
        .map_err(|e| format!("Failed to insert connection: {}", e))?;

        conn.execute(
            "INSERT INTO traffic_daily (date, bytes_sent, connection_count)
             VALUES (?1, ?2, 1)
             ON CONFLICT(date) DO UPDATE SET
                 bytes_sent = bytes_sent + excluded.bytes_sent,
                 connection_count = connection_count + 1",
            rusqlite::params![today, bytes_total as i64],
        )
        .map_err(|e| format!("Failed to upsert traffic_daily: {}", e))?;

        Ok(())
    }

    /// Record a disconnection with full details (connected_at, duration, etc.).
    #[allow(dead_code)]
    pub fn record_disconnection_full(
        &self,
        client_ip: &str,
        client_name: &str,
        connected_at: &str,
        disconnected_at: &str,
        duration_secs: u64,
        bytes_total: u64,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        conn.execute(
            "INSERT INTO connections (client_ip, client_name, connected_at, disconnected_at, duration_secs, bytes_total)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![client_ip, client_name, connected_at, disconnected_at, duration_secs as i64, bytes_total as i64],
        )
        .map_err(|e| format!("Failed to insert connection: {}", e))?;

        conn.execute(
            "INSERT INTO traffic_daily (date, bytes_sent, connection_count)
             VALUES (?1, ?2, 1)
             ON CONFLICT(date) DO UPDATE SET
                 bytes_sent = bytes_sent + excluded.bytes_sent,
                 connection_count = connection_count + 1",
            rusqlite::params![today, bytes_total as i64],
        )
        .map_err(|e| format!("Failed to upsert traffic_daily: {}", e))?;

        Ok(())
    }

    /// Retrieve connection history with pagination.
    pub fn get_connection_history(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT id, client_ip, client_name, connected_at, disconnected_at, duration_secs, bytes_total
                 FROM connections
                 ORDER BY id DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt
            .query_map(rusqlite::params![limit, offset], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, i64>(0)?,
                    "client_ip": row.get::<_, String>(1)?,
                    "client_name": row.get::<_, String>(2)?,
                    "connected_at": row.get::<_, String>(3)?,
                    "disconnected_at": row.get::<_, Option<String>>(4)?,
                    "duration_secs": row.get::<_, i64>(5)?,
                    "bytes_total": row.get::<_, i64>(6)?,
                }))
            })
            .map_err(|e| format!("Failed to query connections: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
        }

        Ok(results)
    }

    /// Retrieve daily traffic statistics for the last N days.
    pub fn get_traffic_stats(
        &self,
        days: u32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT date, bytes_sent, connection_count
                 FROM traffic_daily
                 WHERE date >= date('now', ?1)
                 ORDER BY date DESC",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let days_param = format!("-{} days", days);
        let rows = stmt
            .query_map(rusqlite::params![days_param], |row| {
                Ok(serde_json::json!({
                    "date": row.get::<_, String>(0)?,
                    "bytes_sent": row.get::<_, i64>(1)?,
                    "connection_count": row.get::<_, i64>(2)?,
                }))
            })
            .map_err(|e| format!("Failed to query traffic stats: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
        }

        Ok(results)
    }

    /// Retrieve recent log entries (placeholder — logs are not yet stored in DB).
    pub fn get_logs(
        &self,
        _limit: usize,
    ) -> Result<Vec<serde_json::Value>, String> {
        Ok(vec![])
    }
}
