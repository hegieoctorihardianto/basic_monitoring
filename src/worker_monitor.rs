use crate::telegram::TelegramNotifier;
use crate::email::EmailConfig;
use sqlx::{SqlitePool, Row};
use reqwest;
use tokio::time::{sleep, Duration};
use chrono::{Utc};
use std::sync::Arc;

pub struct MonitorWorker {
    db_pool: Arc<SqlitePool>,
}

impl MonitorWorker {
    pub fn new(db_pool: SqlitePool) -> Self {
        Self {
            db_pool: Arc::new(db_pool),
        }
    }

    pub async fn start(&self) {
        println!("🚀 Starting monitor worker...");

        loop {
            self.check_all_monitors().await;

            // Check every 30 seconds
            sleep(Duration::from_secs(30)).await;
        }
    }

    async fn check_all_monitors(&self) {
        println!("🔍 Checking all monitors...");

        // Get all active monitors
        let rows = match sqlx::query(
            "SELECT id, target_url, type, timeout FROM monitors WHERE is_active = 1"
        )
        .fetch_all(&*self.db_pool)
        .await {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("❌ Error fetching monitors: {}", e);
                return;
            }
        };

        println!("📊 Found {} active monitors to check", rows.len());

        for row in rows {
            let id_result: Result<i64, _> = row.try_get("id");
            let target_url_result: Result<String, _> = row.try_get("target_url");
            let monitor_type_result: Result<String, _> = row.try_get("type");
            let timeout_result: Result<i64, _> = row.try_get("timeout");

            if let (Ok(id), Ok(target_url), Ok(monitor_type), Ok(timeout)) =
                (id_result, target_url_result, monitor_type_result, timeout_result) {
                self.check_monitor(id, &target_url, &monitor_type, timeout).await;
            } else {
                eprintln!("⚠️ Failed to parse monitor row, skipping");
            }
        }
    }

    async fn check_monitor(&self, monitor_id: i64, url: &str, monitor_type: &str, timeout_secs: i64) {
        // Get previous status before check
        let previous_status = self.get_previous_status(monitor_id).await;

        let timeout = Duration::from_secs(timeout_secs as u64);
        let client = match reqwest::Client::builder()
            .timeout(timeout)
            .build() {
                Ok(client) => client,
                Err(e) => {
                    eprintln!("❌ Failed to create HTTP client for monitor {}: {}", monitor_id, e);
                    return;
                }
            };

        let start_time = std::time::Instant::now();

        let result = match monitor_type.to_lowercase().as_str() {
            "http" | "https" => self.check_http(&client, url).await,
            "ping" => self.check_ping(url).await,
            _ => {
                eprintln!("❌ Unknown monitor type: {}", monitor_type);
                return;
            }
        };

        let response_time = start_time.elapsed().as_millis() as i64;

        // Save result to database
        self.save_check_result(monitor_id, &result, response_time).await;

        // ========== CREATE ALERT IN DATABASE ==========
        self.create_alert_if_needed(monitor_id, &result, &previous_status).await;
        
        // ========== RESOLVE ALERT IF BACK ONLINE ==========
        self.resolve_alert_if_needed(monitor_id, &result, &previous_status).await;

        // Send Telegram notification
        self.send_telegram_notification(monitor_id, &result, response_time, previous_status.clone()).await;

        // Send email notification
        self.send_email_notification(monitor_id, &result, response_time, previous_status).await;
    }

    async fn get_previous_status(&self, monitor_id: i64) -> Option<String> {
        match sqlx::query(
            "SELECT status FROM check_results
             WHERE monitor_id = ?
             ORDER BY checked_at DESC
             LIMIT 1"
        )
        .bind(monitor_id)
        .fetch_optional(&*self.db_pool)
        .await
        {
            Ok(Some(row)) => {
                match row.try_get::<String, _>("status") {
                    Ok(status) => Some(status),
                    Err(_) => None,
                }
            }
            _ => None,
        }
    }

    async fn create_alert_if_needed(
        &self,
        monitor_id: i64,
        result: &CheckResult,
        previous_status: &Option<String>,
    ) {
        // Cek apakah status berubah menjadi DOWN
        let is_new_down = result.status == "down" && previous_status.as_deref() != Some("down");
        
        if !is_new_down {
            return;
        }
        
        // Dapatkan user_id dan monitor name
        let monitor_info = sqlx::query!(
            "SELECT user_id, name, target_url FROM monitors WHERE id = ?",
            monitor_id
        )
        .fetch_optional(&*self.db_pool)
        .await;
        
        let (user_id, name, target_url) = match monitor_info {
            Ok(Some(info)) => (info.user_id, info.name, info.target_url),
            _ => return,
        };
        
        // Buat alert di database
        let title = format!("🔴 {} is DOWN!", name);
        let message = format!(
            "Monitor: {}\nURL: {}\nError: {}\nTime: {}",
            name,
            target_url,
            result.error_message.as_deref().unwrap_or("No response"),
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        );
        
        match sqlx::query!(
            "INSERT INTO alerts (user_id, monitor_id, title, message, severity, status, triggered_at)
             VALUES (?, ?, ?, ?, 'critical', 'active', datetime('now'))",
            user_id,
            monitor_id,
            title,
            message
        )
        .execute(&*self.db_pool)
        .await
        {
            Ok(_) => {
                println!("🔔 ALERT created in database for monitor {} (ID: {})", name, monitor_id);
            }
            Err(e) => {
                eprintln!("❌ Failed to create alert for monitor {}: {}", monitor_id, e);
            }
        }
    }

    async fn resolve_alert_if_needed(
        &self,
        monitor_id: i64,
        result: &CheckResult,
        previous_status: &Option<String>,
    ) {
        // Cek apakah status berubah dari DOWN menjadi UP
        let is_recovery = result.status == "up" && previous_status.as_deref() == Some("down");
        
        if !is_recovery {
            return;
        }
        
        // Resolve alert yang aktif
        match sqlx::query!(
            "UPDATE alerts 
             SET status = 'resolved', resolved_at = datetime('now')
             WHERE monitor_id = ? AND status = 'active'",
            monitor_id
        )
        .execute(&*self.db_pool)
        .await
        {
            Ok(updated) => {
                if updated.rows_affected() > 0 {
                    println!("✅ Alert resolved for monitor {} (back ONLINE)", monitor_id);
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to resolve alert for monitor {}: {}", monitor_id, e);
            }
        }
    }

    async fn send_telegram_notification(
        &self,
        monitor_id: i64,
        result: &CheckResult,
        response_time: i64,
        previous_status: Option<String>,
    ) {
        // Hanya kirim notifikasi untuk status "down" atau recovery dari "down"
        let should_notify = match (previous_status.as_deref(), result.status.as_str()) {
            (Some("down"), "up") => true,    // Recovery dari down ke up
            (Some("up"), "down") => true,    // Down dari up
            (Some("slow"), "down") => true,  // Down dari slow
            (None, "down") => true,          // Pertama kali check dan down
            _ => false,
        };

        if !should_notify {
            return;
        }

        // Get monitor details including user_id
        let monitor_details_result = sqlx::query(
            "SELECT m.name, m.target_url, m.user_id
             FROM monitors m
             WHERE m.id = ?"
        )
        .bind(monitor_id)
        .fetch_optional(&*self.db_pool)
        .await;

        let monitor_details = match monitor_details_result {
            Ok(Some(row)) => {
                let name_result = row.try_get::<String, _>("name");
                let target_url_result = row.try_get::<String, _>("target_url");
                let user_id_result = row.try_get::<i32, _>("user_id");

                match (name_result, target_url_result, user_id_result) {
                    (Ok(name), Ok(target_url), Ok(user_id)) => {
                        Some((user_id, name, target_url))
                    }
                    _ => {
                        eprintln!("❌ Failed to get monitor details for monitor {}", monitor_id);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Database error getting monitor details: {}", e);
                None
            }
            _ => None,
        };

        if let Some((user_id, name, target_url)) = monitor_details {
            // Get Telegram config for THIS USER ONLY
            match TelegramNotifier::get_user_config(&self.db_pool, user_id).await {
                Ok(Some(config)) => {
                    // Send notification
                    if let Err(e) = TelegramNotifier::send_monitor_alert(
                        &config,
                        &name,
                        &target_url,
                        &result.status,
                        Some(response_time),
                        result.error_message.as_deref(),
                        previous_status.as_deref(),
                    ).await {
                        eprintln!("❌ Failed to send Telegram notification to user {}: {}", user_id, e);
                    } else {
                        println!("📨 Telegram notification sent to user {} for monitor {} ({} -> {})",
                            user_id,
                            monitor_id,
                            previous_status.as_deref().unwrap_or("None"),
                            result.status
                        );
                    }
                }
                Ok(None) => {
                    // No Telegram config for this user, skip quietly
                    println!("ℹ️ No Telegram config for user {}, skipping notification", user_id);
                }
                Err(e) => {
                    eprintln!("❌ Error getting Telegram config for user {}: {}", user_id, e);
                }
            }
        }
    }

    async fn check_http(&self, client: &reqwest::Client, url: &str) -> CheckResult {
        let mut full_url = url.to_string();
        if !full_url.starts_with("http://") && !full_url.starts_with("https://") {
            full_url = format!("https://{}", url);
        }

        match client.get(&full_url).send().await {
            Ok(response) => {
                let status_code = response.status().as_u16() as i64;

                if status_code >= 200 && status_code < 300 {
                    CheckResult::success("up", Some(status_code), None)
                } else if status_code >= 300 && status_code < 400 {
                    CheckResult::warning("slow", Some(status_code), Some(format!("Redirect: {}", status_code)))
                } else if status_code >= 400 && status_code < 500 {
                    CheckResult::failure("down", Some(status_code), Some(format!("Client error: {}", status_code)))
                } else if status_code >= 500 {
                    CheckResult::failure("down", Some(status_code), Some(format!("Server error: {}", status_code)))
                } else {
                    CheckResult::unknown(Some(status_code), Some("Unknown status code".to_string()))
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Determine error type
                if error_msg.contains("timeout") || error_msg.contains("timed out") {
                    CheckResult::failure("down", None, Some("Timeout".to_string()))
                } else if error_msg.contains("connection refused") || error_msg.contains("failed to connect") {
                    CheckResult::failure("down", None, Some("Connection refused".to_string()))
                } else if error_msg.contains("dns") || error_msg.contains("not known") {
                    CheckResult::failure("down", None, Some("DNS error".to_string()))
                } else if error_msg.contains("tls") || error_msg.contains("ssl") {
                    CheckResult::failure("down", None, Some("SSL/TLS error".to_string()))
                } else {
                    CheckResult::failure("down", None, Some(format!("Network error: {}", error_msg)))
                }
            }
        }
    }

    async fn check_ping(&self, url: &str) -> CheckResult {
        // Remove protocol if present
        let host = url
            .replace("http://", "")
            .replace("https://", "")
            .split('/')
            .next()
            .unwrap_or(url)
            .to_string();

        // Simple ping using system command (for Linux/Mac)
        #[cfg(not(target_os = "windows"))]
        {
            use std::process::Command;

            let output = Command::new("ping")
                .arg("-c")
                .arg("1")
                .arg("-W")
                .arg("1")
                .arg(&host)
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        CheckResult::success("up", None, None)
                    } else {
                        CheckResult::failure("down", None, Some("Ping failed".to_string()))
                    }
                }
                Err(e) => {
                    CheckResult::failure("down", None, Some(format!("Ping error: {}", e)))
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            let output = Command::new("ping")
                .arg("-n")
                .arg("1")
                .arg("-w")
                .arg("1000")
                .arg(&host)
                .output();

            match output {
                Ok(output) => {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if output_str.contains("TTL=") || output_str.contains("Reply from") {
                        CheckResult::success("up", None, None)
                    } else {
                        CheckResult::failure("down", None, Some("Ping failed".to_string()))
                    }
                }
                Err(e) => {
                    CheckResult::failure("down", None, Some(format!("Ping error: {}", e)))
                }
            }
        }
    }

    async fn get_user_email(&self, user_id: i32) -> Option<String> {
        // Cek preferensi user
        let prefs = sqlx::query!(
            "SELECT use_login_email, custom_email, enabled
             FROM email_preferences WHERE user_id = ? AND enabled = 1",
            user_id
        )
        .fetch_optional(&*self.db_pool)
        .await
        .unwrap_or(None);

        if let Some(p) = prefs {
            if p.use_login_email.unwrap_or(true) {
                // Pakai email dari tabel users
                sqlx::query_scalar!("SELECT email FROM users WHERE id = ?", user_id)
                    .fetch_one(&*self.db_pool)
                    .await
                    .ok()
            } else {
                // Pakai custom email
                p.custom_email
            }
        } else {
            // Default: email tidak diaktifkan
            None
        }
    }

    async fn send_email_notification(
        &self,
        monitor_id: i64,
        result: &CheckResult,
        response_time: i64,
        previous_status: Option<String>,
    ) {
        // Hanya kirim notifikasi untuk status "down" atau recovery dari "down"
        let should_notify = match (previous_status.as_deref(), result.status.as_str()) {
            (Some("down"), "up") => true,    // Recovery dari down ke up
            (Some("up"), "down") => true,    // Down dari up
            (Some("slow"), "down") => true,  // Down dari slow
            (None, "down") => true,          // Pertama kali check dan down
            _ => false,
        };

        if !should_notify {
            return;
        }

        // Get monitor details including user_id
        let monitor_details_result = sqlx::query(
            "SELECT m.name, m.target_url, m.user_id
             FROM monitors m
             WHERE m.id = ?"
        )
        .bind(monitor_id)
        .fetch_optional(&*self.db_pool)
        .await;

        let monitor_details = match monitor_details_result {
            Ok(Some(row)) => {
                let name_result = row.try_get::<String, _>("name");
                let target_url_result = row.try_get::<String, _>("target_url");
                let user_id_result = row.try_get::<i32, _>("user_id");

                match (name_result, target_url_result, user_id_result) {
                    (Ok(name), Ok(target_url), Ok(user_id)) => {
                        Some((user_id, name, target_url))
                    }
                    _ => {
                        eprintln!("❌ Failed to get monitor details for monitor {}", monitor_id);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Database error getting monitor details: {}", e);
                None
            }
            _ => None,
        };

        if let Some((user_id, name, _target_url)) = monitor_details {
            // Get user email
            if let Some(user_email) = self.get_user_email(user_id).await {
                // Get email config for this user
                match self.get_email_config(user_id).await {
                    Ok(Some(config)) => {
                        // Send appropriate email based on status change
                        let email_result = if result.status == "down" {
                            config.send_alert(
                                &user_email,
                                &name,
                                result.error_message.as_deref().unwrap_or("Server is down"),
                            )
                        } else {
                            config.send_resolved(
                                &user_email,
                                &name,
                                response_time,
                            )
                        };

                        if let Err(e) = email_result {
                            eprintln!("❌ Failed to send email notification to user {}: {}", user_id, e);
                        } else {
                            println!("✉️ Email notification sent to user {} for monitor {} ({} -> {})",
                                user_id,
                                monitor_id,
                                previous_status.as_deref().unwrap_or("None"),
                                result.status
                            );
                        }
                    }
                    Ok(None) => {
                        // No email config for this user, skip quietly
                        println!("ℹ️ No email config for user {}, skipping notification", user_id);
                    }
                    Err(e) => {
                        eprintln!("❌ Error getting email config for user {}: {}", user_id, e);
                    }
                }
            } else {
                eprintln!("❌ Could not retrieve email for user {}", user_id);
            }
        }
    }

    async fn get_email_config(&self, _user_id: i32) -> Result<Option<EmailConfig>, sqlx::Error> {
        // Kita pakai konfigurasi GLOBAL dari .env
        Ok(Some(EmailConfig::default_from_env()))
    }

    async fn save_check_result(&self, monitor_id: i64, result: &CheckResult, response_time: i64) {
        let checked_at = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        match sqlx::query(
            "INSERT INTO check_results (monitor_id, status, response_time, status_code, error_message, checked_at)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(monitor_id)
        .bind(&result.status)
        .bind(response_time)
        .bind(result.status_code)
        .bind(&result.error_message)
        .bind(&checked_at)
        .execute(&*self.db_pool)
        .await {
            Ok(_) => {
                // Silent success
            }
            Err(e) => {
                eprintln!("❌ Error saving check result: {}", e);
            }
        }
    }
}

#[derive(Clone)]
pub struct CheckResult {
    pub status: String,
    pub status_code: Option<i64>,
    pub error_message: Option<String>,
}

impl CheckResult {
    pub fn success(status: &str, status_code: Option<i64>, message: Option<String>) -> Self {
        Self {
            status: status.to_string(),
            status_code,
            error_message: message,
        }
    }

    pub fn warning(status: &str, status_code: Option<i64>, message: Option<String>) -> Self {
        Self {
            status: status.to_string(),
            status_code,
            error_message: message,
        }
    }

    pub fn failure(status: &str, status_code: Option<i64>, message: Option<String>) -> Self {
        Self {
            status: status.to_string(),
            status_code,
            error_message: message,
        }
    }

    pub fn unknown(status_code: Option<i64>, message: Option<String>) -> Self {
        Self {
            status: "unknown".to_string(),
            status_code,
            error_message: message,
        }
    }
}

pub async fn start_monitor_worker(db_pool: SqlitePool) {
    let worker = MonitorWorker::new(db_pool);

    tokio::spawn(async move {
        worker.start().await;
    });

    println!("🎯 Monitor worker started in background");
}
