use reqwest;
use serde_json::json;
use std::time::Duration;
use sqlx::{SqlitePool, Row};
use chrono::Local;
use std::env;

#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
    pub enabled: bool,
}

pub struct TelegramNotifier;

impl TelegramNotifier {
    // Ambil konfigurasi Telegram untuk user TERTENTU
    pub async fn get_user_config(
        db_pool: &SqlitePool,
        user_id: i32,
    ) -> Result<Option<TelegramConfig>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT telegram_chat_id, telegram_enabled
            FROM notification_settings
            WHERE user_id = ? AND telegram_enabled = 1
            "#
        )
        .bind(user_id)  // FILTER BY SPECIFIC USER
        .fetch_optional(db_pool)
        .await?;

        match row {
            Some(r) => {
                let chat_id: String = r.try_get("telegram_chat_id")?;
                let enabled: bool = r.try_get("telegram_enabled")?;

                if chat_id.is_empty() {
                    Ok(None)
                } else {
                    // PAKAI GLOBAL BOT TOKEN (dari .env)
                    let global_token = env::var("TELEGRAM_GLOBAL_BOT_TOKEN")
                        .unwrap_or_else(|_| "8572929961:AAGg52v7JK9v5SHrkHKmheX9eV6EBmo8uRQ".to_string());

                    Ok(Some(TelegramConfig {
                        bot_token: global_token,
                        chat_id,
                        enabled,
                    }))
                }
            }
            None => Ok(None),
        }
    }

    // Simpan konfigurasi Telegram (TIDAK simpan bot_token)
    pub async fn save_config(
        db_pool: &SqlitePool,
        user_id: i32,
        chat_id: &str,
        enabled: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO notification_settings
            (user_id, telegram_chat_id, telegram_enabled)
            VALUES (?, ?, ?)
            "#
        )
        .bind(user_id)
        .bind(chat_id)
        .bind(enabled)
        .execute(db_pool)
        .await?;

        Ok(())
    }

    // Hapus konfigurasi Telegram untuk user
    pub async fn delete_config(
        db_pool: &SqlitePool,
        user_id: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM notification_settings WHERE user_id = ?"
        )
        .bind(user_id)
        .execute(db_pool)
        .await?;

        Ok(())
    }

    // Kirim pesan ke Telegram
    pub async fn send_message(
        config: &TelegramConfig,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !config.enabled {
            return Ok(());
        }

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            config.bot_token
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let payload = json!({
            "chat_id": config.chat_id,
            "text": message,
            "parse_mode": "HTML"
        });

        let response = client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            println!("[{}] ✅ Telegram message sent to chat_id: {}",
                Local::now().format("%H:%M:%S"),
                config.chat_id);
        } else {
            let error_text = response.text().await?;
            eprintln!("[{}] ❌ Failed to send Telegram to {}: {}",
                Local::now().format("%H:%M:%S"),
                config.chat_id,
                error_text);
        }

        Ok(())
    }

    // Kirim notifikasi status monitor
    pub async fn send_monitor_alert(
        config: &TelegramConfig,
        monitor_name: &str,
        url: &str,
        status: &str,
        response_time: Option<i64>,
        error_message: Option<&str>,
        previous_status: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !config.enabled {
            return Ok(());
        }

        // Hanya kirim jika status berubah ke/from "down"
        let should_send = match (previous_status, status) {
            (Some("down"), "up") => true,    // Recovery
            (Some("up"), "down") => true,    // Down
            (Some("slow"), "down") => true,  // Slow → Down
            (None, "down") => true,          // First check is down
            _ => false,
        };

        if !should_send {
            return Ok(());
        }

        let emoji = match status {
            "up" => "✅",
            "down" => "🔴",
            "slow" => "🟡",
            _ => "⚪",
        };

        let status_text = match status {
            "up" => "UP",
            "down" => "DOWN",
            "slow" => "SLOW",
            _ => "UNKNOWN",
        };

        let time_str = match response_time {
            Some(ms) => format!("{}ms", ms),
            None => "N/A".to_string(),
        };

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

        let mut message = format!(
            "{} <b>{} Alert</b>\n",
            emoji, status_text
        );

        message.push_str(&format!("<b>Monitor:</b> {}\n", monitor_name));
        message.push_str(&format!("<b>URL:</b> {}\n", url));
        message.push_str(&format!("<b>Response:</b> {}\n", time_str));

        if let Some(err) = error_message {
            message.push_str(&format!("<b>Error:</b> {}\n", err));
        }

        if let Some(prev) = previous_status {
            if prev != status {
                message.push_str(&format!("\n🔄 Status changed from {} to {}\n",
                    prev.to_uppercase(), status_text));
            }
        }

        message.push_str(&format!("\n<code>{}</code>", timestamp));

        Self::send_message(config, &message).await
    }
}
