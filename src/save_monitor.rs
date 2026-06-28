// save_monitor.rs
use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::SqlitePool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct MonitorForm {
    pub names: Vec<String>,
    pub urls: Vec<String>,
    pub types: Vec<String>,
    pub intervals: Vec<String>,
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/monitors", web::post().to(create_monitor))
    );
}

async fn create_monitor(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    form: web::Form<MonitorForm>,
) -> impl Responder {
    // Debug: print form data
    println!("Form received: names={:?}, urls={:?}, types={:?}, intervals={:?}", 
             form.names, form.urls, form.types, form.intervals);
    
    // Check session
    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(id)) => {
            println!("User ID from session: {}", id);
            id
        },
        Ok(None) => {
            println!("No user_id in session");
            return HttpResponse::Unauthorized().body(
                r##"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                    <div class="flex items-center">
                        <svg class="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                        </svg>
                        <span class="font-medium text-red-800">Session Expired</span>
                    </div>
                    <p class="mt-1 text-sm text-red-600">Please login again.</p>
                </div>"##
            );
        },
        Err(e) => {
            println!("Session error: {:?}", e);
            return HttpResponse::Unauthorized().body("Session error");
        }
    };
    
    // Validate input
    if form.names.is_empty() || form.urls.is_empty() {
        return HttpResponse::Ok().body(
            r##"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                <div class="flex items-center">
                    <svg class="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                    </svg>
                    <span class="font-medium text-red-800">Validation Error</span>
                </div>
                <p class="mt-1 text-sm text-red-600">Please provide at least one monitor name and URL.</p>
            </div>"##
        );
    }
    
    let mut success_count = 0;
    let mut error_messages = Vec::new();
    
    // Process each monitor
    for i in 0..form.names.len() {
        if i < form.names.len() && i < form.urls.len() && i < form.types.len() && i < form.intervals.len() {
            let name = form.names[i].trim();
            let url = form.urls[i].trim();
            let monitor_type = form.types[i].trim();
            let interval = form.intervals[i].parse::<i64>().unwrap_or(60);
            
            // Validate
            if name.is_empty() {
                error_messages.push(format!("Monitor #{}: Name cannot be empty", i + 1));
                continue;
            }
            
            if url.is_empty() {
                error_messages.push(format!("Monitor #{}: URL cannot be empty", i + 1));
                continue;
            }
            
            // Insert into database
            match sqlx::query(
                "INSERT INTO monitors (user_id, name, target_url, type, check_interval, is_active) 
                 VALUES (?, ?, ?, ?, ?, 1)"
            )
            .bind(user_id)
            .bind(name)
            .bind(url)
            .bind(monitor_type)
            .bind(interval)
            .execute(db_pool.get_ref())
            .await
            {
                Ok(result) => {
                    println!("Inserted monitor '{}' with ID: {}", name, result.last_insert_rowid());
                    success_count += 1;
                }
                Err(e) => {
                    println!("Database error for monitor '{}': {:?}", name, e);
                    error_messages.push(format!("Monitor #{}: {}", i + 1, e));
                }
            }
        }
    }
    
    if success_count > 0 {
        let success_html = if error_messages.is_empty() {
            format!(
                r##"<div class="p-4 bg-green-50 border border-green-200 rounded-lg">
                    <div class="flex items-center">
                        <svg class="w-5 h-5 text-green-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"/>
                        </svg>
                        <span class="font-medium text-green-800">Success!</span>
                    </div>
                    <p class="mt-1 text-sm text-green-600">{} monitor(s) added successfully.</p>
                </div>"##,
                success_count
            )
        } else {
            format!(
                r##"<div class="p-4 bg-yellow-50 border border-yellow-200 rounded-lg">
                    <div class="flex items-center">
                        <svg class="w-5 h-5 text-yellow-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                            <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                        </svg>
                        <span class="font-medium text-yellow-800">Partial Success</span>
                    </div>
                    <p class="mt-1 text-sm text-yellow-700">
                        {} monitor(s) added successfully.<br>
                        Errors: {}
                    </p>
                </div>"##,
                success_count,
                error_messages.join("<br>")
            )
        };
        
        HttpResponse::Ok().body(success_html)
    } else {
        HttpResponse::Ok().body(format!(
            r##"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                <div class="flex items-center">
                    <svg class="w-5 h-5 text-red-500 mr-2" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                    </svg>
                    <span class="font-medium text-red-800">Error adding monitors</span>
                </div>
                <p class="mt-1 text-sm text-red-600">{}</p>
            </div>"##,
            error_messages.join("<br>")
        ))
    }
}