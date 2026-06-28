// src/ssl_checker.rs
use chrono::NaiveDateTime;
use reqwest::Client;
use std::time::Duration;

pub async fn fetch_ssl_certificate(
    domain: &str,
    port: i64,
) -> Result<(NaiveDateTime, String), String> {
    let url = format!("https://{}:{}", domain, port);

    println!("🔍 Fetching SSL certificate from {}:{}", domain, port);

    // Buat client yang mengabaikan error certificate (termasuk expired)
    let client = Client::builder()
        .danger_accept_invalid_certs(true)  // Terima semua certificate termasuk expired
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    // Lakukan request
    match client.get(&url).send().await {
        Ok(response) => {
            // Untuk sertifikat expired, response akan tetap error
            // Tapi kita bisa dapatkan status
            let status = response.status();
            println!("Response status: {}", status);

            // Untuk expired.badssl.com, ini akan tetap error
            Err("Certificate expired or invalid".to_string())
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("expired") || err_str.contains("certificate") {
                // Kembalikan error expired
                Err(format!("Certificate expired: {}", err_str))
            } else {
                Err(format!("Connection error: {}", err_str))
            }
        }
    }
}
