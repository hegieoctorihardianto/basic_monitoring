// src/email.rs
use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::authentication::Credentials;
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::client::{Tls, TlsParameters};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_email: String,
    pub from_name: String,
    pub encryption: String,
}

impl EmailConfig {
    /// Membaca konfigurasi dari environment variable (DEFAULT)
    pub fn default_from_env() -> Self {
        EmailConfig {
            smtp_host: env::var("SMTP_HOST").unwrap_or_else(|_| "mail.bisnismania.com".to_string()),
            smtp_port: env::var("SMTP_PORT").unwrap_or_else(|_| "465".to_string()).parse().unwrap_or(465),
            smtp_username: env::var("SMTP_USERNAME").unwrap_or_else(|_| "info@bisnismania.com".to_string()),
            smtp_password: env::var("SMTP_PASSWORD").unwrap_or_else(|_| "rentaljasa990".to_string()),
            from_email: env::var("FROM_EMAIL").unwrap_or_else(|_| "info@bisnismania.com".to_string()),
            from_name: env::var("FROM_NAME").unwrap_or_else(|_| "Basic Monitoring Pro".to_string()),
            encryption: env::var("SMTP_ENCRYPTION").unwrap_or_else(|_| "ssl".to_string()),
        }
    }

    /// Kirim email
    pub fn send_email(&self, to: &str, subject: &str, html_body: &str) -> Result<String, String> {
        println!("📧 Sending email to: {}", to);
        println!("   Subject: {}", subject);
        println!("   From: {} <{}>", self.from_name, self.from_email);
        println!("   Server: {}:{} ({})", self.smtp_host, self.smtp_port, self.encryption);

        // Parse email addresses
        let from: Mailbox = match format!("{} <{}>", self.from_name, self.from_email).parse() {
            Ok(m) => m,
            Err(e) => {
                let msg = format!("❌ Invalid from email: {}", e);
                eprintln!("{}", msg);
                return Err(msg);
            }
        };

        let to_mailbox: Mailbox = match to.parse() {
            Ok(m) => m,
            Err(e) => {
                let msg = format!("❌ Invalid to email: {}", e);
                eprintln!("{}", msg);
                return Err(msg);
            }
        };

        // Build email
        let email = match Message::builder()
            .from(from)
            .to(to_mailbox.clone())
            .subject(subject)
            .multipart(MultiPart::alternative_plain_html(
                "Please enable HTML to view this email".to_string(),
                html_body.to_string(),
            )) {
                Ok(m) => m,
                Err(e) => {
                    let msg = format!("❌ Failed to build email: {}", e);
                    eprintln!("{}", msg);
                    return Err(msg);
                }
            };

        // Setup credentials
        let creds = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

        // Buat TLS parameters dengan versi yang benar
        let tls_parameters = match TlsParameters::builder(self.smtp_host.clone())
            .dangerous_accept_invalid_certs(false)
            .build() {
                Ok(params) => params,
                Err(e) => {
                    let msg = format!("❌ Failed to create TLS parameters: {}", e);
                    eprintln!("{}", msg);
                    return Err(msg);
                }
            };

        // Create transport dengan TLS yang benar
        let result = if self.encryption == "ssl" || self.encryption == "tls" {
            // Port 465 (SSL/TLS)
            SmtpTransport::relay(&self.smtp_host)
                .map_err(|e| format!("Failed to create TLS transport: {}", e))?
                .port(self.smtp_port)
                .credentials(creds)
                .tls(Tls::Required(tls_parameters))
                .build()
                .send(&email)
        } else {
            // Port 587 (STARTTLS) atau lainnya
            SmtpTransport::starttls_relay(&self.smtp_host)
                .map_err(|e| format!("Failed to create STARTTLS transport: {}", e))?
                .port(self.smtp_port)
                .credentials(creds)
                .tls(Tls::Required(tls_parameters))
                .build()
                .send(&email)
        };

        match result {
            Ok(_) => {
                println!("✅ Email sent successfully to {}", to);
                Ok("Email sent successfully".to_string())
            }
            Err(e) => {
                let msg = format!("❌ Failed to send email: {}", e);
                eprintln!("{}", msg);
                Err(msg)
            }
        }
    }

    /// Kirim alert (dipanggil dari worker_monitor)
    pub fn send_alert(&self, to: &str, monitor_name: &str, error_msg: &str) -> Result<String, String> {
        let subject = format!("🔴 DOWN: {}", monitor_name);
        let html_body = format!(r##"
        <!DOCTYPE html>
        <html>
        <head>
            <style>
                body {{ font-family: Arial, sans-serif; background: #f8fafc; padding: 20px; }}
                .container {{ max-width: 600px; margin: 0 auto; background: white; border-radius: 8px; padding: 30px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
                .critical {{ color: #ef4444; font-size: 24px; font-weight: bold; margin-bottom: 20px; }}
                .info {{ color: #64748b; line-height: 1.6; }}
                .monitor {{ background: #f1f5f9; padding: 15px; border-radius: 6px; margin: 20px 0; }}
                .monitor-name {{ font-weight: bold; color: #0f172a; }}
            </style>
        </head>
        <body>
            <div class="container">
                <div class="critical">🔴 Alert: Server Down!</div>
                <div class="monitor">
                    <span class="monitor-name">Monitor:</span> {}<br>
                    <span class="monitor-name">Error:</span> {}
                </div>
                <div class="info">
                    <p>Server tidak meresponse. Tim IT sedang menangani masalah ini.</p>
                    <p>Waktu: {}</p>
                </div>
            </div>
        </body>
        </html>
        "##, monitor_name, error_msg, chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));

        self.send_email(to, &subject, &html_body)
    }

    /// Kirim resolved notification
    pub fn send_resolved(&self, to: &str, monitor_name: &str, response_time: i64) -> Result<String, String> {
        let subject = format!("✅ RESOLVED: {}", monitor_name);
        let html_body = format!(r##"
        <!DOCTYPE html>
        <html>
        <head>
            <style>
                body {{ font-family: Arial, sans-serif; background: #f8fafc; padding: 20px; }}
                .container {{ max-width: 600px; margin: 0 auto; background: white; border-radius: 8px; padding: 30px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
                .success {{ color: #10b981; font-size: 24px; font-weight: bold; margin-bottom: 20px; }}
                .info {{ color: #64748b; line-height: 1.6; }}
                .monitor {{ background: #f1f5f9; padding: 15px; border-radius: 6px; margin: 20px 0; }}
                .monitor-name {{ font-weight: bold; color: #0f172a; }}
            </style>
        </head>
        <body>
            <div class="container">
                <div class="success">✅ Resolved: Server Kembali Normal</div>
                <div class="monitor">
                    <span class="monitor-name">Monitor:</span> {}<br>
                    <span class="monitor-name">Response Time:</span> {}ms
                </div>
                <div class="info">
                    <p>Server sudah kembali online dan meresponse dengan normal.</p>
                    <p>Waktu: {}</p>
                </div>
            </div>
        </body>
        </html>
        "##, monitor_name, response_time, chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));

        self.send_email(to, &subject, &html_body)
    }
}

/// Helper function untuk ngecek config di env
pub fn validate_env() -> bool {
    let host = env::var("SMTP_HOST").unwrap_or_default();
    let user = env::var("SMTP_USERNAME").unwrap_or_default();
    let pass = env::var("SMTP_PASSWORD").unwrap_or_default();

    if host.is_empty() || user.is_empty() || pass.is_empty() {
        eprintln!("⚠️  Email SMTP not configured in .env file");
        false
    } else {
        println!("✅ Email SMTP configured for {}", user);
        true
    }
}
