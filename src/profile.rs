use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::{SqlitePool, Row};
use serde::{Deserialize, Serialize};
use bcrypt::{hash, verify, DEFAULT_COST};

// INCLUDE SEPERTI PHP
use crate::navbar;
use crate::sidebar;
use crate::footer;

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    current_password: String,
    new_password: String,
    confirm_password: String,
}

#[derive(Deserialize)]
pub struct UpdateProfileForm {
    full_name: String,
    timezone: Option<String>,
    email_notifications: Option<bool>,
    alert_on_down: Option<bool>,
    alert_on_resolved: Option<bool>,
    daily_report: Option<bool>,
    weekly_report: Option<bool>,
}

#[derive(Serialize)]
struct UserProfile {
    id: i32,
    email: String,
    full_name: String,
    timezone: String,
    email_notifications: bool,
    alert_on_down: bool,
    alert_on_resolved: bool,
    daily_report: bool,
    weekly_report: bool,
    created_at: String,
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/profile")
            .route("", web::get().to(profile_page))
            .route("/update", web::post().to(update_profile))
            .route("/change-password", web::post().to(change_password))
    );
}

async fn profile_page(session: Session, db_pool: web::Data<SqlitePool>) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => {
            render_profile(&email, &db_pool).await
        }
        _ => {
            HttpResponse::Found()
                .append_header(("Location", "/"))
                .finish()
        }
    }
}

async fn render_profile(email: &str, db_pool: &SqlitePool) -> HttpResponse {
    let mut html = String::new();

    // Get user profile from database - PAKE QUERY BIASA
    let user_row = sqlx::query(
        "SELECT id, email, full_name, created_at FROM users WHERE email = ?"
    )
    .bind(email)
    .fetch_optional(db_pool)
    .await
    .unwrap_or(None);

    if user_row.is_none() {
        return HttpResponse::Found()
            .append_header(("Location", "/"))
            .finish();
    }

    let row = user_row.unwrap();
    let user_id: i32 = row.get("id");
    let user_email: String = row.get("email");
    let user_full_name: String = row.get("full_name");
    let user_created_at: Option<String> = row.get("created_at");

    // Get user preferences - PAKE QUERY BIASA
    let prefs_row = sqlx::query(
        "SELECT timezone, email_notifications, alert_on_down,
                alert_on_resolved, daily_report, weekly_report
         FROM user_preferences WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_optional(db_pool)
    .await
    .unwrap_or(None);

    let (timezone, email_notifications, alert_on_down, alert_on_resolved, daily_report, weekly_report) =
        if let Some(pref) = prefs_row {
            (
                match pref.try_get::<String, _>("timezone") {
                    Ok(val) => val,
                    Err(_) => "Asia/Jakarta".to_string(),
                },
                match pref.try_get::<i64, _>("email_notifications") {
                    Ok(val) => val == 1,
                    Err(_) => true,
                },
                match pref.try_get::<i64, _>("alert_on_down") {
                    Ok(val) => val == 1,
                    Err(_) => true,
                },
                match pref.try_get::<i64, _>("alert_on_resolved") {
                    Ok(val) => val == 1,
                    Err(_) => true,
                },
                match pref.try_get::<i64, _>("daily_report") {
                    Ok(val) => val == 1,
                    Err(_) => false,
                },
                match pref.try_get::<i64, _>("weekly_report") {
                    Ok(val) => val == 1,
                    Err(_) => false,
                },
            )
        } else {
            (
                "Asia/Jakarta".to_string(),
                true,
                true,
                true,
                false,
                false,
            )
        };

    let created_at_display = user_created_at.unwrap_or_else(|| "Unknown".to_string());

    // DOCTYPE DAN HEAD
    html.push_str(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Profile - Basic Monitoring</title>
    <link href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/sweetalert2@11"></script>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');

        body {
            font-family: 'Inter', sans-serif;
            background-color: #f8fafc;
        }

        .sidebar-bg {
            background: linear-gradient(180deg, #1e293b 0%, #0f172a 100%);
        }

        .profile-card {
            transition: transform 0.2s ease;
        }

        .profile-card:hover {
            transform: translateY(-2px);
        }

        .tab-button {
            transition: all 0.2s ease;
        }

        .tab-button.active {
            border-bottom: 2px solid #3b82f6;
            color: #3b82f6;
        }

        .loading-spinner {
            display: inline-block;
            width: 20px;
            height: 20px;
            border: 2px solid #e5e7eb;
            border-top-color: #3b82f6;
            border-radius: 50%;
            animation: spin 1s linear infinite;
        }

        @keyframes spin {
            to { transform: rotate(360deg); }
        }
    </style>
</head>
<body>"##);

    // INCLUDE NAVBAR
    html.push_str(&navbar::render(email));

    // WRAPPER
    html.push_str(r#"<div class="flex">"#);

    // INCLUDE SIDEBAR
    html.push_str(&sidebar::render());

    // KONTEN UTAMA PROFILE
    html.push_str(&format!(r##"
        <!-- Main Content -->
        <div class="flex-1 p-6">
            <!-- Header -->
            <div class="flex justify-between items-center mb-6">
                <div>
                    <h1 class="text-2xl font-bold text-gray-800">My Profile</h1>
                    <p class="text-gray-600">Manage your account settings and preferences</p>
                </div>
                <div class="flex space-x-3">
                    <span class="px-3 py-1 bg-green-100 text-green-800 rounded-full text-sm">
                        Member since {created_at}
                    </span>
                </div>
            </div>

            <!-- Tabs -->
            <div class="border-b border-gray-200 mb-6">
                <nav class="flex space-x-8">
                    <button onclick="switchTab('profile')" id="tab-profile" class="tab-button active py-3 px-1 text-sm font-medium border-b-2 border-blue-500 text-blue-600">
                        Profile Information
                    </button>
                    <button onclick="switchTab('security')" id="tab-security" class="tab-button py-3 px-1 text-sm font-medium text-gray-500 hover:text-gray-700">
                        Security
                    </button>
                    <button onclick="switchTab('preferences')" id="tab-preferences" class="tab-button py-3 px-1 text-sm font-medium text-gray-500 hover:text-gray-700">
                        Preferences
                    </button>
                </nav>
            </div>

            <!-- Profile Tab -->
            <div id="profile-tab" class="tab-content">
                <div class="bg-white rounded-lg shadow p-6">
                    <h2 class="text-lg font-semibold mb-4">Personal Information</h2>

                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Email</label>
                            <input type="email" value="{email}"
                                   class="w-full px-4 py-2 rounded-lg border border-gray-300 bg-gray-50"
                                   readonly>
                            <p class="text-sm text-gray-500 mt-1">Email cannot be changed</p>
                        </div>

                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Full Name</label>
                            <input type="text" id="full-name" value="{full_name}"
                                   class="w-full px-4 py-2 rounded-lg border border-gray-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                        </div>

                        <div class="pt-4">
                            <button onclick="updateProfile()"
                                    class="px-6 py-2 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700">
                                Update Profile
                            </button>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Security Tab (Hidden by default) -->
            <div id="security-tab" class="tab-content hidden">
                <div class="bg-white rounded-lg shadow p-6">
                    <h2 class="text-lg font-semibold mb-4">Change Password</h2>

                    <form id="change-password-form" class="space-y-4 max-w-md">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Current Password</label>
                            <input type="password" id="current-password"
                                   class="w-full px-4 py-2 rounded-lg border border-gray-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                        </div>

                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">New Password</label>
                            <input type="password" id="new-password"
                                   class="w-full px-4 py-2 rounded-lg border border-gray-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                            <p class="text-sm text-gray-500 mt-1">Min 8 characters with letters and numbers</p>
                        </div>

                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Confirm New Password</label>
                            <input type="password" id="confirm-password"
                                   class="w-full px-4 py-2 rounded-lg border border-gray-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                        </div>

                        <div class="pt-4">
                            <button type="button" onclick="changePassword()"
                                    class="px-6 py-2 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700">
                                Change Password
                            </button>
                        </div>
                    </form>
                </div>
            </div>

            <!-- Preferences Tab (Hidden by default) -->
            <div id="preferences-tab" class="tab-content hidden">
                <div class="bg-white rounded-lg shadow p-6">
                    <h2 class="text-lg font-semibold mb-4">Notification Preferences</h2>

                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-2">Timezone</label>
                            <select id="timezone" class="w-full md:w-64 px-4 py-2 rounded-lg border border-gray-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                                <option value="Asia/Jakarta" {timezone_sel_jakarta}>Asia/Jakarta (WIB)</option>
                                <option value="Asia/Makassar" {timezone_sel_makassar}>Asia/Makassar (WITA)</option>
                                <option value="Asia/Jayapura" {timezone_sel_jayapura}>Asia/Jayapura (WIT)</option>
                                <option value="UTC" {timezone_sel_utc}>UTC</option>
                            </select>
                        </div>

                        <div class="pt-2">
                            <h3 class="font-medium mb-3">Email Notifications</h3>

                            <div class="space-y-2">
                                <label class="flex items-center">
                                    <input type="checkbox" id="email-notifications" {email_notif_checked}
                                           class="rounded border-gray-300 text-blue-600 focus:ring-blue-500">
                                    <span class="ml-2 text-sm text-gray-700">Enable email notifications</span>
                                </label>

                                <label class="flex items-center ml-6">
                                    <input type="checkbox" id="alert-on-down" {alert_down_checked}
                                           class="rounded border-gray-300 text-blue-600 focus:ring-blue-500">
                                    <span class="ml-2 text-sm text-gray-700">Alert when server goes DOWN</span>
                                </label>

                                <label class="flex items-center ml-6">
                                    <input type="checkbox" id="alert-on-resolved" {alert_resolved_checked}
                                           class="rounded border-gray-300 text-blue-600 focus:ring-blue-500">
                                    <span class="ml-2 text-sm text-gray-700">Alert when server RECOVERED</span>
                                </label>
                            </div>
                        </div>

                        <div class="pt-2">
                            <h3 class="font-medium mb-3">Email Reports</h3>

                            <div class="space-y-2">
                                <label class="flex items-center">
                                    <input type="checkbox" id="daily-report" {daily_report_checked}
                                           class="rounded border-gray-300 text-blue-600 focus:ring-blue-500">
                                    <span class="ml-2 text-sm text-gray-700">Send daily summary report</span>
                                </label>

                                <label class="flex items-center">
                                    <input type="checkbox" id="weekly-report" {weekly_report_checked}
                                           class="rounded border-gray-300 text-blue-600 focus:ring-blue-500">
                                    <span class="ml-2 text-sm text-gray-700">Send weekly performance report</span>
                                </label>
                            </div>
                        </div>

                        <div class="pt-4">
                            <button onclick="savePreferences()"
                                    class="px-6 py-2 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700">
                                Save Preferences
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
    "##,
    created_at = created_at_display,
    email = user_email,
    full_name = user_full_name,
    timezone_sel_jakarta = if timezone == "Asia/Jakarta" { "selected" } else { "" },
    timezone_sel_makassar = if timezone == "Asia/Makassar" { "selected" } else { "" },
    timezone_sel_jayapura = if timezone == "Asia/Jayapura" { "selected" } else { "" },
    timezone_sel_utc = if timezone == "UTC" { "selected" } else { "" },
    email_notif_checked = if email_notifications { "checked" } else { "" },
    alert_down_checked = if alert_on_down { "checked" } else { "" },
    alert_resolved_checked = if alert_on_resolved { "checked" } else { "" },
    daily_report_checked = if daily_report { "checked" } else { "" },
    weekly_report_checked = if weekly_report { "checked" } else { "" },
    ));

    // INCLUDE FOOTER
    html.push_str(&footer::render());

    // JAVASCRIPT
    html.push_str(r##"
    <script>
        function switchTab(tabName) {
            // Update tab buttons
            document.getElementById('tab-profile').classList.remove('active', 'text-blue-600', 'border-blue-500');
            document.getElementById('tab-security').classList.remove('active', 'text-blue-600', 'border-blue-500');
            document.getElementById('tab-preferences').classList.remove('active', 'text-blue-600', 'border-blue-500');

            document.getElementById('tab-profile').classList.add('text-gray-500');
            document.getElementById('tab-security').classList.add('text-gray-500');
            document.getElementById('tab-preferences').classList.add('text-gray-500');

            // Hide all tabs
            document.getElementById('profile-tab').classList.add('hidden');
            document.getElementById('security-tab').classList.add('hidden');
            document.getElementById('preferences-tab').classList.add('hidden');

            // Show selected tab
            if (tabName === 'profile') {
                document.getElementById('tab-profile').classList.add('active', 'text-blue-600', 'border-blue-500');
                document.getElementById('profile-tab').classList.remove('hidden');
            } else if (tabName === 'security') {
                document.getElementById('tab-security').classList.add('active', 'text-blue-600', 'border-blue-500');
                document.getElementById('security-tab').classList.remove('hidden');
            } else if (tabName === 'preferences') {
                document.getElementById('tab-preferences').classList.add('active', 'text-blue-600', 'border-blue-500');
                document.getElementById('preferences-tab').classList.remove('hidden');
            }
        }

        function updateProfile() {
            const fullName = document.getElementById('full-name').value;

            fetch('/profile/update', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    full_name: fullName
                })
            })
            .then(response => response.json())
            .then(data => {
                if (data.success) {
                    Swal.fire({
                        icon: 'success',
                        title: 'Success',
                        text: 'Profile updated successfully',
                        timer: 2000,
                        showConfirmButton: false
                    });
                } else {
                    Swal.fire({
                        icon: 'error',
                        title: 'Error',
                        text: data.message
                    });
                }
            })
            .catch(error => {
                Swal.fire({
                    icon: 'error',
                    title: 'Error',
                    text: 'Failed to update profile'
                });
            });
        }

        function changePassword() {
            const current = document.getElementById('current-password').value;
            const newPass = document.getElementById('new-password').value;
            const confirm = document.getElementById('confirm-password').value;

            if (!current || !newPass || !confirm) {
                Swal.fire({
                    icon: 'error',
                    title: 'Error',
                    text: 'Please fill all fields'
                });
                return;
            }

            if (newPass !== confirm) {
                Swal.fire({
                    icon: 'error',
                    title: 'Error',
                    text: 'New passwords do not match'
                });
                return;
            }

            if (newPass.length < 8) {
                Swal.fire({
                    icon: 'error',
                    title: 'Error',
                    text: 'Password must be at least 8 characters'
                });
                return;
            }

            fetch('/profile/change-password', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    current_password: current,
                    new_password: newPass,
                    confirm_password: confirm
                })
            })
            .then(response => response.json())
            .then(data => {
                if (data.success) {
                    Swal.fire({
                        icon: 'success',
                        title: 'Success',
                        text: 'Password changed successfully',
                        timer: 2000,
                        showConfirmButton: false
                    }).then(() => {
                        document.getElementById('current-password').value = '';
                        document.getElementById('new-password').value = '';
                        document.getElementById('confirm-password').value = '';
                    });
                } else {
                    Swal.fire({
                        icon: 'error',
                        title: 'Error',
                        text: data.message
                    });
                }
            })
            .catch(error => {
                Swal.fire({
                    icon: 'error',
                    title: 'Error',
                    text: 'Failed to change password'
                });
            });
        }

        function savePreferences() {
            const prefs = {
                timezone: document.getElementById('timezone').value,
                email_notifications: document.getElementById('email-notifications').checked,
                alert_on_down: document.getElementById('alert-on-down').checked,
                alert_on_resolved: document.getElementById('alert-on-resolved').checked,
                daily_report: document.getElementById('daily-report').checked,
                weekly_report: document.getElementById('weekly-report').checked
            };

            fetch('/profile/update', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(prefs)
            })
            .then(response => response.json())
            .then(data => {
                if (data.success) {
                    Swal.fire({
                        icon: 'success',
                        title: 'Success',
                        text: 'Preferences saved',
                        timer: 2000,
                        showConfirmButton: false
                    });
                } else {
                    Swal.fire({
                        icon: 'error',
                        title: 'Error',
                        text: data.message
                    });
                }
            });
        }
    </script>
    </body>
    </html>
    "##);

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

// ==================== HANDLERS ====================

async fn update_profile(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    form: web::Json<UpdateProfileForm>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            // Update user full name
            let update_user = sqlx::query(
                "UPDATE users SET full_name = ? WHERE id = ?"
            )
            .bind(&form.full_name)
            .bind(user_id)
            .execute(db_pool.get_ref())
            .await;

            if update_user.is_err() {
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "success": false,
                    "message": "Failed to update user"
                }));
            }

            // Insert or update preferences
            let prefs = sqlx::query(
                "INSERT OR REPLACE INTO user_preferences
                (user_id, timezone, email_notifications, alert_on_down,
                 alert_on_resolved, daily_report, weekly_report)
                VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(user_id)
            .bind(form.timezone.as_deref().unwrap_or("Asia/Jakarta"))
            .bind(form.email_notifications.unwrap_or(false))
            .bind(form.alert_on_down.unwrap_or(true))
            .bind(form.alert_on_resolved.unwrap_or(true))
            .bind(form.daily_report.unwrap_or(false))
            .bind(form.weekly_report.unwrap_or(false))
            .execute(db_pool.get_ref())
            .await;

            match prefs {
                Ok(_) => HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "message": "Profile updated successfully"
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

async fn change_password(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    form: web::Json<ChangePasswordForm>,
) -> impl Responder {
    if form.new_password != form.confirm_password {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "message": "New passwords do not match"
        }));
    }

    if form.new_password.len() < 8 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "message": "Password must be at least 8 characters"
        }));
    }

    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            // Get current password hash
            let user_row = sqlx::query(
                "SELECT password_hash FROM users WHERE id = ?"
            )
            .bind(user_id)
            .fetch_optional(db_pool.get_ref())
            .await;

            match user_row {
                Ok(Some(row)) => {
                    let password_hash: String = row.get("password_hash");

                    // Verify current password
                    match verify(&form.current_password, &password_hash) {
                        Ok(true) => {
                            // Hash new password
                            match hash(&form.new_password, DEFAULT_COST) {
                                Ok(new_hash) => {
                                    let update = sqlx::query(
                                        "UPDATE users SET password_hash = ? WHERE id = ?"
                                    )
                                    .bind(new_hash)
                                    .bind(user_id)
                                    .execute(db_pool.get_ref())
                                    .await;

                                    match update {
                                        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
                                            "success": true,
                                            "message": "Password changed successfully"
                                        })),
                                        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                                            "success": false,
                                            "message": format!("Database error: {}", e)
                                        }))
                                    }
                                }
                                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                                    "success": false,
                                    "message": format!("Failed to hash password: {}", e)
                                }))
                            }
                        }
                        Ok(false) => HttpResponse::BadRequest().json(serde_json::json!({
                            "success": false,
                            "message": "Current password is incorrect"
                        })),
                        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                            "success": false,
                            "message": format!("Password verification error: {}", e)
                        }))
                    }
                }
                Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "message": "User not found"
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
