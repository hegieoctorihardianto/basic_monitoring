use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use actix_files::Files;
use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use dotenv::dotenv;
use std::env;
use serde::Deserialize;
use sqlx::SqlitePool;
use bcrypt::verify;

mod database;
mod register;
mod dashboard;
mod monitors;
mod save_monitor;
mod worker_monitor;
mod reports;
mod alerts;
mod email_preferences;
mod ssl_monitors;
mod ssl_checker;
mod email_worker;
mod telegram;
mod ssl_worker;
mod telegram_setup;
mod profile;
mod server_health;
mod public_status;

// Tambahkan mod ini:
mod navbar;
mod sidebar;
mod footer;

#[derive(Deserialize)]
struct LoginForm {
    email: String,
    password: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    println!("🚀 Starting BasicMonitoring (Standalone) on port {}...", port);

    let db_pool = database::init().await;

    // Start monitor worker
    worker_monitor::start_monitor_worker(db_pool.clone()).await;

    // Start SSL worker
    ssl_worker::start_ssl_worker(db_pool.clone()).await;

    println!("✅ Database connected");

    let db_data = web::Data::new(db_pool);

    // Secret key for cookies (in production, use proper key from env)
    let secret_key = Key::generate();

    HttpServer::new(move || {
        App::new()
            // Session middleware
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    secret_key.clone(),
                )
                .cookie_secure(false) // Set true in production with HTTPS
                .build()
            )
            .app_data(db_data.clone())
            .service(Files::new("/static", "./static"))
            .route("/", web::get().to(home_page))
            .route("/login", web::post().to(login_submit))
            .route("/logout", web::get().to(logout))
            .configure(register::routes)
            .configure(dashboard::routes)
            .configure(monitors::routes)
            .configure(save_monitor::routes)
            .configure(email_preferences::routes)
            .configure(ssl_monitors::routes)
            .configure(telegram_setup::routes)
            .configure(alerts::routes)
            .configure(reports::routes)
            .configure(profile::routes)
            .configure(server_health::routes)
            .configure(public_status::routes)


    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}

async fn home_page(session: Session) -> impl Responder {
    // Check if user is already logged in
    if let Ok(Some(_)) = session.get::<String>("email") {
        // Redirect to dashboard if already logged in
        return HttpResponse::Found()
            .append_header(("Location", "/dashboard"))
            .finish();
    }

    // Show login page if not logged in
    let html = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Basic Monitoring - Professional Edition</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script src="/static/js/htmx.min.js"></script>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
        body { font-family: 'Inter', sans-serif; background-color: #0f172a; }
        .login-gradient { background: linear-gradient(135deg, #1e293b 0%, #0f172a 100%); }
    </style>
</head>
<body class="login-gradient min-h-screen">
    <div class="min-h-screen flex items-center justify-center p-4">
        <div class="w-full max-w-md">
            <div class="mb-8 text-center">
                <div class="inline-flex items-center justify-center w-16 h-16 bg-white rounded-2xl mb-4">
                    <span class="text-3xl text-slate-800">📊</span>
                </div>
                <h1 class="text-3xl font-bold text-white">BasicMonitoring</h1>
                <p class="text-slate-300 mt-2">Professional Edition</p>
            </div>

            <div class="bg-white rounded-2xl shadow-2xl p-8">
                <h2 class="text-2xl font-bold text-slate-800 mb-6">Admin Login</h2>
                <form hx-post="/login" hx-target="#login-result" hx-swap="innerHTML" class="space-y-5">
                    <div>
                        <label class="block text-slate-700 font-medium mb-2">Email</label>
                        <input type="email" name="email"
                               class="w-full px-4 py-3 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                               placeholder="admin@company.com" required>
                    </div>

                    <div>
                        <label class="block text-slate-700 font-medium mb-2">Password</label>
                        <input type="password" name="password"
                               class="w-full px-4 py-3 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                               placeholder="••••••••" required>
                    </div>

                    <button type="submit"
                            class="w-full bg-slate-800 text-white font-semibold py-3 rounded-lg hover:bg-slate-900">
                        Sign In
                    </button>

                    <div id="login-result" class="mt-4"></div>
                </form>

                <div class="mt-6 pt-6 border-t border-slate-200">
                    <p class="text-center text-slate-600">
                        Need an account?
                        <a href="/register" class="text-blue-600 font-semibold hover:text-blue-800">Create admin account</a>
                    </p>
                </div>
            </div>
        </div>
    </div>
</body>
</html>"##;

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

async fn login_submit(
    form: web::Form<LoginForm>,
    db_pool: web::Data<SqlitePool>,
    session: Session,
) -> impl Responder {
    println!("🔐 Login attempt for: {}", form.email);

    // Query database for user
    match sqlx::query!(
        "SELECT id, email, full_name, password_hash FROM users WHERE email = ?",
        form.email
    )
    .fetch_optional(db_pool.get_ref())
    .await
    {
        Ok(Some(user)) => {
            println!("✅ User found in DB: {}", user.email);

            match verify(&form.password, &user.password_hash) {
                Ok(true) => {
                    println!("✅ Password verified!");

                    // Set session data
                    session.insert("user_id", user.id).unwrap();
                    session.insert("email", user.email).unwrap();
                    session.insert("full_name", user.full_name).unwrap();

                    let success = r##"<div class="p-4 bg-green-50 border border-green-200 rounded-lg">
                        <div class="flex items-center">
                            <svg class="w-5 h-5 text-green-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"/>
                            </svg>
                            <span class="font-medium text-green-800">Login successful!</span>
                        </div>
                        <p class="mt-1 text-sm text-green-600">Welcome back! Redirecting to dashboard...</p>
                    </div>
                    <script>
                        setTimeout(() => {
                            window.location.href = '/dashboard';
                        }, 1000);
                    </script>"##;

                    HttpResponse::Ok().body(success)
                }
                Ok(false) => {
                    println!("❌ Password incorrect");
                    let error = r##"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                        <div class="flex items-center">
                            <svg class="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                            </svg>
                            <span class="font-medium text-red-800">Invalid password</span>
                        </div>
                        <p class="mt-1 text-sm text-red-600">The password you entered is incorrect.</p>
                    </div>"##;

                    HttpResponse::Ok().body(error)
                }
                Err(e) => {
                    println!("❌ Bcrypt error: {:?}", e);
                    let error = r##"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                        <div class="flex items-center">
                            <svg class="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                            </svg>
                            <span class="font-medium text-red-800">System error</span>
                        </div>
                        <p class="mt-1 text-sm text-red-600">Failed to verify password. Please try again.</p>
                    </div>"##;

                    HttpResponse::Ok().body(error)
                }
            }
        }
        Ok(None) => {
            println!("❌ User not found: {}", form.email);
            let error = r##"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                <div class="flex items-center">
                    <svg class="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                    </svg>
                    <span class="font-medium text-red-800">Account not found</span>
                </div>
                <p class="mt-1 text-sm text-red-600">No account found with this email address.</p>
                <div class="mt-3">
                    <a href="/register" class="text-sm text-blue-600 hover:text-blue-800">Create a new account</a>
                </div>
            </div>"##;

            HttpResponse::Ok().body(error)
        }
        Err(e) => {
            println!("❌ Database error: {:?}", e);
            let error = r##"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                <div class="flex items-center">
                    <svg class="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                    </svg>
                    <span class="font-medium text-red-800">System error</span>
                </div>
                <p class="mt-1 text-sm text-red-600">Database connection error. Please try again.</p>
            </div>"##;

            HttpResponse::Ok().body(error)
        }
    }
}

async fn logout(session: Session) -> impl Responder {
    session.purge();
    HttpResponse::Found()
        .append_header(("Location", "/"))
        .finish()
}
