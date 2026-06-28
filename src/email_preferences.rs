use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::SqlitePool;
use serde::Deserialize;

// INCLUDE SEPERTI PHP
use crate::navbar;
use crate::sidebar;
use crate::footer;

#[derive(Deserialize)]
pub struct EmailPrefsForm {
    use_login_email: bool,
    custom_email: String,
    enabled: bool,
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/email-preferences")
            .route("", web::get().to(email_prefs_page))
            .route("/save", web::post().to(save_email_prefs))
    );
}

async fn email_prefs_page(session: Session, db_pool: web::Data<SqlitePool>) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => render_email_prefs(&email, &db_pool).await,
        _ => HttpResponse::Found()
            .append_header(("Location", "/"))
            .finish(),
    }
}

async fn render_email_prefs(email: &str, db_pool: &SqlitePool) -> HttpResponse {
    let mut html = String::new();

    // Get user_id from email
    let user_id = sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(db_pool)
        .await
        .unwrap_or(None);

    if user_id.is_none() {
        return HttpResponse::Found()
            .append_header(("Location", "/"))
            .finish();
    }

    let user_id_val = user_id.unwrap();

    // Get preferences from database
    let prefs = sqlx::query!(
        "SELECT use_login_email, custom_email, enabled FROM email_preferences WHERE user_id = ?",
        user_id_val
    )
    .fetch_optional(db_pool)
    .await
    .unwrap_or(None);

    let (use_login_email, custom_email, enabled) = if let Some(p) = prefs {
        (
            p.use_login_email.unwrap_or(true),
            p.custom_email.unwrap_or_default(),
            p.enabled.unwrap_or(false)
        )
    } else {
        (true, "".to_string(), false)
    };

    // DOCTYPE DAN HEAD
    html.push_str(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Email Notifications - Basic Monitoring</title>
    <link href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/sweetalert2@11"></script>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
        body { font-family: 'Inter', sans-serif; background-color: #f8fafc; }
        .sidebar-bg { background: linear-gradient(180deg, #1e293b 0%, #0f172a 100%); }
        .pref-card { transition: all 0.2s ease; }
        .pref-card:hover { box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.1); }
        .info-box { background-color: #eff6ff; border-left: 4px solid #3b82f6; }
    </style>
</head>
<body>"##);

    // INCLUDE NAVBAR
    html.push_str(&navbar::render(email));

    // WRAPPER
    html.push_str(r#"<div class="flex">"#);

    // INCLUDE SIDEBAR
    html.push_str(&sidebar::render());

    // KONTEN UTAMA
    let content = format!(r##"
        <div class="flex-1 p-6">
            <div class="mb-8">
                <h1 class="text-3xl font-bold text-gray-800">📧 Email Notifications</h1>
                <p class="text-gray-600 mt-2">Configure where email notifications will be sent</p>
            </div>

            <!-- Info Box -->
            <div class="info-box rounded-lg p-4 mb-6 max-w-2xl">
                <div class="flex">
                    <div class="flex-shrink-0">
                        <svg class="h-5 w-5 text-blue-400" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
                            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2h-1V9z" clip-rule="evenodd" />
                        </svg>
                    </div>
                    <div class="ml-3">
                        <p class="text-sm text-blue-700">
                            Notifications will be sent through <strong>info@bisnismania.com</strong>.
                            No SMTP configuration needed, just select your destination email below.
                        </p>
                    </div>
                </div>
            </div>

            <!-- Preferences Card -->
            <div class="bg-white rounded-lg shadow p-6 max-w-2xl pref-card">
                <form id="email-prefs-form">
                    <div class="mb-6">
                        <label class="flex items-center">
                            <input type="checkbox" id="enabled" {} class="w-5 h-5 rounded border-gray-300 text-blue-600 focus:ring-blue-500">
                            <span class="ml-3 text-gray-700 font-medium">Enable email notifications</span>
                        </label>
                        <p class="mt-1 text-sm text-gray-500 ml-8">
                            If enabled, you will receive emails whenever a server goes down or recovers
                        </p>
                    </div>

                    <div class="space-y-4 mb-6">
                        <div class="border rounded-lg p-4 {}">
                            <label class="flex items-center">
                                <input type="radio" name="email_option" value="login" {} class="w-4 h-4 text-blue-600 focus:ring-blue-500">
                                <span class="ml-3">
                                    <span class="font-medium text-gray-700">Send to my login email</span>
                                    <p class="text-sm text-gray-500">{}</p>
                                </span>
                            </label>
                        </div>

                        <div class="border rounded-lg p-4 {}">
                            <label class="flex items-center">
                                <input type="radio" name="email_option" value="custom" {} class="w-4 h-4 text-blue-600 focus:ring-blue-500">
                                <span class="ml-3 font-medium text-gray-700">Send to a different email</span>
                            </label>
                            <div class="mt-3 ml-7">
                                <input type="email" id="custom_email" value="{}"
                                       class="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                                       placeholder="email@domain.com" {} >
                                <p class="mt-1 text-xs text-gray-500">Enter destination email address</p>
                            </div>
                        </div>
                    </div>

                    <div class="flex justify-end pt-4 border-t">
                        <button type="button" onclick="savePreferences()"
                                class="px-6 py-2 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700 transition flex items-center">
                            <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                            </svg>
                            Save Settings
                        </button>
                    </div>
                </form>
            </div>

            <!-- Preview Card -->
            <div class="mt-6 max-w-2xl">
                <details class="bg-gray-50 rounded-lg p-4">
                    <summary class="text-sm font-medium text-gray-700 cursor-pointer">📋 Example notifications you will receive</summary>
                    <div class="mt-4 space-y-4">
                        <div class="bg-red-50 border border-red-200 rounded-lg p-4">
                            <p class="text-red-800 font-medium">🔴 DOWN: My Website</p>
                            <p class="text-red-600 text-sm mt-1">Error: Connection timeout</p>
                            <p class="text-gray-500 text-xs mt-2">Time: 14:30:45</p>
                        </div>
                        <div class="bg-green-50 border border-green-200 rounded-lg p-4">
                            <p class="text-green-800 font-medium">✅ RESOLVED: My Website</p>
                            <p class="text-green-600 text-sm mt-1">Response Time: 234ms</p>
                            <p class="text-gray-500 text-xs mt-2">Time: 14:35:22</p>
                        </div>
                    </div>
                </details>
            </div>
        </div>
    </div>
    "##,
    if enabled { "checked" } else { "" },
    if use_login_email { "border-blue-200 bg-blue-50" } else { "border-gray-200" },
    if use_login_email { "checked" } else { "" },
    email,
    if !use_login_email { "border-blue-200 bg-blue-50" } else { "border-gray-200" },
    if !use_login_email { "checked" } else { "" },
    custom_email,
    if use_login_email { "disabled" } else { "" }
    );

    html.push_str(&content);

    // INCLUDE FOOTER
    html.push_str(&footer::render());

    // JAVASCRIPT
    html.push_str(r##"
    <script>
        function savePreferences() {
            const useLoginEmail = document.querySelector('input[name="email_option"]:checked')?.value === 'login';

            const data = {
                use_login_email: useLoginEmail,
                custom_email: document.getElementById('custom_email').value,
                enabled: document.getElementById('enabled').checked
            };

            // Validation
            if (!useLoginEmail && !data.custom_email) {
                Swal.fire({
                    icon: 'error',
                    title: 'Email Required',
                    text: 'Please enter a destination email'
                });
                return;
            }

            // Show loading
            const saveBtn = event.target;
            const originalText = saveBtn.innerHTML;
            saveBtn.innerHTML = '<div class="loading-spinner mr-2"></div> Saving...';
            saveBtn.disabled = true;

            fetch('/email-preferences/save', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(data)
            })
            .then(res => res.json())
            .then(data => {
                if (data.success) {
                    Swal.fire({
                        icon: 'success',
                        title: 'Success!',
                        text: 'Email settings saved',
                        timer: 2000,
                        showConfirmButton: false
                    }).then(() => {
                        location.reload();
                    });
                } else {
                    Swal.fire({
                        icon: 'error',
                        title: 'Failed',
                        text: data.message
                    });
                }
            })
            .catch(error => {
                Swal.fire({
                    icon: 'error',
                    title: 'Error',
                    text: error.message
                });
            })
            .finally(() => {
                saveBtn.innerHTML = originalText;
                saveBtn.disabled = false;
            });
        }

        // Add event listener for radio buttons
        document.querySelectorAll('input[name="email_option"]').forEach(radio => {
            radio.addEventListener('change', function() {
                const customInput = document.getElementById('custom_email');
                if (this.value === 'custom') {
                    customInput.disabled = false;
                    customInput.classList.remove('bg-gray-100');
                } else {
                    customInput.disabled = true;
                    customInput.classList.add('bg-gray-100');
                }
            });
        });

        // Trigger on page load
        window.addEventListener('DOMContentLoaded', function() {
            const selectedOption = document.querySelector('input[name="email_option"]:checked');
            const customInput = document.getElementById('custom_email');
            if (selectedOption && selectedOption.value === 'custom') {
                customInput.disabled = false;
                customInput.classList.remove('bg-gray-100');
            } else {
                customInput.disabled = true;
                customInput.classList.add('bg-gray-100');
            }
        });
    </script>
    </body>
    </html>
    "##);

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

// ==================== HANDLER ====================

async fn save_email_prefs(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    form: web::Json<EmailPrefsForm>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let result = sqlx::query(
                "INSERT OR REPLACE INTO email_preferences (user_id, use_login_email, custom_email, enabled)
                 VALUES (?, ?, ?, ?)"
            )
            .bind(user_id)
            .bind(form.use_login_email)
            .bind(&form.custom_email)
            .bind(form.enabled)
            .execute(db_pool.get_ref())
            .await;

            match result {
                Ok(_) => HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "message": "Email settings saved successfully"
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "success": false,
                    "message": format!("Database error: {}", e)
                }))
            }
        }
        _ => HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Not authenticated"
        }))
    }
}