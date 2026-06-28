// src/ssl_worker.rs
use crate::ssl_checker::fetch_ssl_certificate;
use sqlx::SqlitePool;
use std::time::Duration;
use tokio::time;
use chrono::Utc;

pub async fn start_ssl_worker(db_pool: SqlitePool) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(3600)); // Cek setiap 1 jam

        println!("🔄 SSL Worker started - checking certificates periodically");

        loop {
            interval.tick().await;

            // Ambil semua monitor aktif
            let monitors = sqlx::query!(
                "SELECT id, domain, port, check_interval, last_checked
                 FROM ssl_monitors
                 WHERE is_active = 1"
            )
            .fetch_all(&db_pool)
            .await;

            match monitors {
                Ok(monitors) => {
                    for monitor in monitors {
                        let interval_hours = monitor.check_interval.unwrap_or(24) as i64;
                        let should_check = match monitor.last_checked {
                            Some(last_checked) => {
                                let hours_since = (Utc::now().naive_utc() - last_checked).num_hours();
                                hours_since >= interval_hours
                            }
                            None => true,
                        };

                        if should_check {
                            let port_value = monitor.port.unwrap_or(443);
                            println!("🔄 Auto-checking SSL for {}:{}", monitor.domain, port_value);

                            match fetch_ssl_certificate(&monitor.domain, port_value).await {
                                Ok((expiry_date, issuer)) => {
                                    let now = Utc::now().naive_utc();
                                    let days_left = (expiry_date - now).num_days();
                                    let status = if expiry_date > now {
                                        if days_left < 7 { "expiring" } else { "valid" }
                                    } else { "expired" };

                                    match sqlx::query!(
                                        "UPDATE ssl_monitors
                                         SET expiry_date = ?,
                                             issuer = ?,
                                             last_checked = CURRENT_TIMESTAMP,
                                             last_status = ?
                                         WHERE id = ?",
                                        expiry_date,
                                        issuer,
                                        status,
                                        monitor.id
                                    )
                                    .execute(&db_pool)
                                    .await
                                    {
                                        Ok(_) => {
                                            println!("✅ Auto-updated {}: {} ({} days left)",
                                                monitor.domain, status, days_left);

                                            // Buat alert jika expired atau expiring
                                            if days_left <= 7 && days_left >= 0 {
                                                let user_id = sqlx::query_scalar!(
                                                    "SELECT user_id FROM ssl_monitors WHERE id = ?",
                                                    monitor.id
                                                )
                                                .fetch_one(&db_pool)
                                                .await
                                                .unwrap_or(0);

                                                let existing_alert = sqlx::query_scalar!(
                                                    "SELECT COUNT(*) FROM ssl_alerts
                                                     WHERE ssl_monitor_id = ?
                                                     AND status != 'resolved'
                                                     AND triggered_at >= datetime('now', '-1 day')",
                                                    monitor.id
                                                )
                                                .fetch_one(&db_pool)
                                                .await
                                                .unwrap_or(0);

                                                if existing_alert == 0 {
                                                    let (severity, title, message) = if days_left <= 0 {
                                                        ("critical",
                                                         "⚠️ SSL Certificate EXPIRED",
                                                         format!("SSL certificate for {} has EXPIRED! Renew immediately.", monitor.domain))
                                                    } else if days_left <= 3 {
                                                        ("critical",
                                                         "🔴 SSL Certificate Expiring IMMINENTLY",
                                                         format!("SSL certificate for {} expires in {} days! Action required.", monitor.domain, days_left))
                                                    } else {
                                                        ("warning",
                                                         "🟡 SSL Certificate Expiring Soon",
                                                         format!("SSL certificate for {} will expire in {} days. Please renew soon.", monitor.domain, days_left))
                                                    };

                                                    let _ = sqlx::query!(
                                                        "INSERT INTO ssl_alerts (user_id, ssl_monitor_id, title, message, severity, status)
                                                         VALUES (?, ?, ?, ?, ?, 'active')",
                                                        user_id,
                                                        monitor.id,
                                                        title,
                                                        message,
                                                        severity
                                                    )
                                                    .execute(&db_pool)
                                                    .await;

                                                    println!("🔔 SSL Alert created for {}: {}", monitor.domain, title);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("❌ Failed to update {}: {}", monitor.domain, e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    // HANYA LOG error, JANGAN buat alert
                                    println!("⚠️ Auto-check failed for {}: {}", monitor.domain, e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ SSL Worker error: {}", e);
                }
            }
        }
    });
}
