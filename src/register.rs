use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use sqlx::SqlitePool;
use bcrypt::{hash, DEFAULT_COST};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
pub struct RegisterForm {
    full_name: String,
    email: String,
    password: String,
    confirm_password: String,
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/register")
            .route("", web::get().to(register_page))
            .route("", web::post().to(register_submit))
    );
}

// Generate random string
fn generate_random_string(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut result = String::with_capacity(length);
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    
    for _ in 0..length {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let idx = (seed as usize) % CHARSET.len();
        result.push(CHARSET[idx] as char);
    }
    result
}

fn render_page(title: &str, content: &str) -> String {
    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script src="/static/js/htmx.min.js"></script>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
        body {{ font-family: 'Inter', sans-serif; background-color: #0f172a; }}
        .gradient-bg {{ background: linear-gradient(135deg, #1e293b 0%, #0f172a 100%); }}
    </style>
</head>
<body class="gradient-bg min-h-screen">
    <div class="min-h-screen flex items-center justify-center p-4">
        <div class="w-full max-w-md">
            <div class="mb-8 text-center">
                <div class="inline-flex items-center justify-center w-16 h-16 bg-white rounded-2xl mb-4">
                    <span class="text-3xl text-slate-800">📊</span>
                </div>
                <h1 class="text-3xl font-bold text-white">BasicMonitoring</h1>
                <p class="text-slate-300 mt-2">Professional Edition</p>
            </div>
            {}
        </div>
    </div>
</body>
</html>"##, title, content)
}

async fn register_page() -> impl Responder {
    let form_html = r##"<div class="bg-white rounded-2xl shadow-2xl p-8">
        <h2 class="text-2xl font-bold text-slate-800 mb-2">Create Admin Account</h2>
        <p class="text-slate-600 mb-6">Set up your monitoring dashboard</p>
        
        <form hx-post="/register" hx-target="#result" hx-swap="innerHTML" class="space-y-5">
            <div>
                <label class="block text-slate-700 font-medium mb-2">Full Name</label>
                <input type="text" name="full_name" 
                       class="w-full px-4 py-3 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200 outline-none"
                       placeholder="John Smith" required>
            </div>
            
            <div>
                <label class="block text-slate-700 font-medium mb-2">Email Address</label>
                <input type="email" name="email" 
                       class="w-full px-4 py-3 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200 outline-none"
                       placeholder="admin@yourcompany.com" required>
            </div>
            
            <div>
                <label class="block text-slate-700 font-medium mb-2">Password</label>
                <input type="password" name="password" 
                       class="w-full px-4 py-3 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200 outline-none"
                       placeholder="Minimum 8 characters" required>
                <p class="text-sm text-slate-500 mt-1">Use a strong password</p>
            </div>
            
            <div>
                <label class="block text-slate-700 font-medium mb-2">Confirm Password</label>
                <input type="password" name="confirm_password" 
                       class="w-full px-4 py-3 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200 outline-none"
                       placeholder="Re-enter your password" required>
            </div>
            
            <div class="flex items-center">
                <input type="checkbox" id="terms" name="terms" class="h-4 w-4 text-blue-600 rounded" required>
                <label for="terms" class="ml-2 text-slate-700 text-sm">
                    I agree to the Terms of Service
                </label>
            </div>
            
            <button type="submit" 
                    class="w-full bg-gradient-to-r from-slate-800 to-slate-900 text-white font-semibold py-3 rounded-lg hover:shadow-lg">
                Create Admin Account
            </button>
            
            <div id="result" class="mt-4"></div>
        </form>
        
        <div class="mt-6 pt-6 border-t border-slate-200">
            <p class="text-center text-slate-600">
                Already have an account? 
                <a href="/" class="text-blue-600 font-semibold hover:text-blue-800">Sign in here</a>
            </p>
        </div>
    </div>"##;
    
    let html = render_page("Register - Basic Monitoring", form_html);
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn register_submit(
    form: web::Form<RegisterForm>,
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    // Validasi input
    if form.password.len() < 8 {
        return HttpResponse::Ok().body(
            r##"<div class="p-4 bg-yellow-50 border border-yellow-200 rounded-lg">
                <div class="flex items-center">
                    <svg class="w-5 h-5 text-yellow-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                    </svg>
                    <span class="font-medium text-yellow-800">Password too short</span>
                </div>
                <p class="mt-1 text-sm text-yellow-600">Password must be at least 8 characters long.</p>
            </div>"##
        );
    }
    
    // ✅ PERBAIKAN DI SINI: confish_password -> confirm_password
    if form.password != form.confirm_password {
        return HttpResponse::Ok().body(
            r##"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                <div class="flex items-center">
                    <svg class="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                    </svg>
                    <span class="font-medium text-red-800">Passwords don't match</span>
                </div>
                <p class="mt-1 text-sm text-red-600">Please make sure both passwords are identical.</p>
            </div>"##
        );
    }
    
    // Hash password
    let password_hash = match hash(&form.password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(_) => {
            return HttpResponse::Ok().body(
                r##"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                    <div class="flex items-center">
                        <svg class="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                        </svg>
                        <span class="font-medium text-red-800">System error</span>
                    </div>
                    <p class="mt-1 text-sm text-red-600">Failed to process password. Please try again.</p>
                </div>"##
            );
        }
    };
    
    // Generate UUID dan license key
    let user_uuid = generate_random_string(32);
    let license_key = format!("LIC-{}", generate_random_string(12));
    
    // ✅ PERBAIKAN: Gunakan query() bukan query!() dan datetime('now')
    let result = sqlx::query(
        r#"
        INSERT INTO users (uuid, full_name, email, password_hash, license_key, created_at)
        VALUES (?, ?, ?, ?, ?, datetime('now'))
        "#,
    )
    .bind(&user_uuid)
    .bind(&form.full_name)
    .bind(&form.email)
    .bind(&password_hash)
    .bind(&license_key)
    .execute(db_pool.get_ref())
    .await;
    
    match result {
        Ok(_) => {
            let success_html = r##"<div class="p-4 bg-green-50 border border-green-200 rounded-lg">
                <div class="flex items-center">
                    <svg class="w-5 h-5 text-green-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"/>
                    </svg>
                    <span class="font-medium text-green-800">Registration successful!</span>
                </div>
                <p class="mt-1 text-sm text-green-600">Your admin account has been created.</p>
                <div class="mt-4">
                    <a href="/" 
                       class="inline-block w-full text-center bg-blue-600 text-white font-semibold py-3 rounded-lg hover:bg-blue-700 transition">
                        Go to Login
                    </a>
                </div>
            </div>"##;
            
            HttpResponse::Ok().body(success_html)
        }
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("UNIQUE constraint failed") => {
            HttpResponse::Ok().body(
                r##"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                    <div class="flex items-center">
                        <svg class="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                        </svg>
                        <span class="font-medium text-red-800">Email already registered</span>
                    </div>
                    <p class="mt-1 text-sm text-red-600">This email address is already in use.</p>
                </div>"##
            )
        }
        Err(e) => {
            println!("Database error: {:?}", e);
            HttpResponse::Ok().body(
                r##"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                    <div class="flex items-center">
                        <svg class="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                        </svg>
                        <span class="font-medium text-red-800">Registration failed</span>
                    </div>
                    <p class="mt-1 text-sm text-red-600">Database error. Please try again.</p>
                </div>"##
            )
        }
    }
}