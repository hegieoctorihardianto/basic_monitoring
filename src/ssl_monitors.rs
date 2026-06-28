// src/ssl_monitors.rs
use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::{SqlitePool, Row};
use serde::{Deserialize, Serialize};
use chrono::{Utc, NaiveDateTime};

// INCLUDE SEPERTI PHP
use crate::navbar;
use crate::sidebar;
use crate::footer;
use crate::ssl_checker::fetch_ssl_certificate;

// ==================== STRUCTS ====================

#[derive(Debug, sqlx::FromRow)]
pub struct SslMonitor {
    pub id: i64,
    pub user_id: i64,
    pub domain: String,
    pub port: i64,
    pub check_interval: i64,
    pub alert_days: String,
    pub last_checked: Option<String>,
    pub last_status: Option<String>,
    pub expiry_date: Option<NaiveDateTime>,
    pub issuer: Option<String>,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct SslMonitorRow {
    pub id: i64,
    pub domain: String,
    pub port: i64,
    pub status: String,
    pub status_class: String,
    pub days_left: i64,
    pub expiry_date: String,
    pub issuer: String,
    pub is_active: bool,
    pub last_checked: Option<String>,
}

#[derive(Serialize)]
struct SslStatus {
    id: i64,
    domain: String,
    status: String,
    days_left: i64,
    expiry_date: Option<String>,
}

// ==================== QUERY PARAMS ====================

#[derive(Deserialize)]
pub struct SslMonitorQuery {
    pub page: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct SslMonitorJson {
    pub monitors: Vec<SslMonitorData>,
}

#[derive(Deserialize, Debug)]
pub struct SslMonitorData {
    pub domain: String,
    pub port: Option<i64>,
    pub check_interval: Option<i64>,
    pub alert_days: Option<String>,
}

#[derive(Deserialize)]
struct StatusQuery {
    ids: String,
}

// ==================== HELPER FUNCTIONS ====================

fn format_last_checked(last_checked: NaiveDateTime) -> String {
    let now = Utc::now().naive_utc();
    let duration = now - last_checked;

    let seconds = duration.num_seconds();
    if seconds < 60 {
        "Just now".to_string()
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        if minutes == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", minutes)
        }
    } else if seconds < 86400 {
        let hours = seconds / 3600;
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else {
        let days = seconds / 86400;
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days)
        }
    }
}


// ==================== ROUTES ====================

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/ssl-monitors")
            .route("", web::get().to(ssl_monitors_page))
            .route("", web::post().to(create_ssl_monitor_json))
            .route("/data", web::get().to(ssl_monitors_data))
            .route("/{id}/toggle", web::post().to(toggle_ssl_monitor))
            .route("/{id}/delete", web::post().to(delete_ssl_monitor))
            .route("/{id}/check", web::post().to(check_ssl_now))
            .route("/status-updates", web::get().to(ssl_status_updates))
    );
}

// ==================== PAGE RENDER ====================

async fn ssl_monitors_page(session: Session) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => render_ssl_monitors_page(&email),
        _ => HttpResponse::Found()
            .append_header(("Location", "/"))
            .finish(),
    }
}

fn render_ssl_monitors_page(email: &str) -> HttpResponse {
    let mut html = String::new();

    html.push_str(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>SSL Monitors - Basic Monitoring</title>
    <link href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/sweetalert2@11"></script>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
        body { font-family: 'Inter', sans-serif; background-color: #f8fafc; }
        .sidebar-bg { background: linear-gradient(180deg, #1e293b 0%, #0f172a 100%); }
        .status-valid { color: #10b981; }
        .status-expiring { color: #f59e0b; }
        .status-expired { color: #ef4444; }
        .status-unknown { color: #6b7280; }
        .badge-valid { background-color: #d1fae5; color: #065f46; }
        .badge-expiring { background-color: #fef3c7; color: #92400e; }
        .badge-expired { background-color: #fee2e2; color: #991b1b; }
        .badge-unknown { background-color: #f3f4f6; color: #1f2937; }
        .switch { position: relative; display: inline-block; width: 60px; height: 34px; }
        .switch input { opacity: 0; width: 0; height: 0; }
        .slider { position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: #ccc; transition: .4s; }
        .slider:before { position: absolute; content: ""; height: 26px; width: 26px; left: 4px; bottom: 4px; background-color: white; transition: .4s; }
        input:checked + .slider { background-color: #10b981; }
        input:checked + .slider:before { transform: translateX(26px); }
        .slider.round { border-radius: 34px; }
        .slider.round:before { border-radius: 50%; }
        .loading-indicator { display: none; }
        .loading .loading-indicator { display: flex; }
        .modal-overlay {
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background-color: rgba(0, 0, 0, 0.5);
            display: none;
            justify-content: center;
            align-items: center;
            z-index: 50;
        }
        .modal-overlay.active { display: flex; }
        .btn-check {
            transition: all 0.2s ease;
        }
        .btn-check:hover {
            transform: scale(1.1);
        }
    </style>
</head>
<body>"##);

    html.push_str(&navbar::render(email));
    html.push_str(r#"<div class="flex">"#);
    html.push_str(&sidebar::render());

    html.push_str(r##"
    <div class="flex-1 p-6">
        <div class="flex flex-col md:flex-row justify-between items-start md:items-center mb-8">
            <div>
                <h1 class="text-3xl font-bold text-slate-800">🔒 SSL Certificate Monitors</h1>
                <p class="text-slate-600 mt-2">Monitor SSL certificate expiry and get alerts before they expire</p>
            </div>
            <div class="mt-4 md:mt-0">
                <button onclick="openAddSslModal()"
                        class="inline-flex items-center bg-blue-600 text-white font-semibold px-5 py-2.5 rounded-lg hover:bg-blue-700 transition">
                    <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
                    </svg>
                    Add SSL Monitor
                </button>
            </div>
        </div>

        <!-- Filters -->
        <div class="bg-white rounded-xl shadow p-4 mb-6">
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div>
                    <label class="block text-slate-700 text-sm font-medium mb-2">Search</label>
                    <input type="text" id="search-input" placeholder="Search by domain..."
                           class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                </div>
                <div>
                    <label class="block text-slate-700 text-sm font-medium mb-2">Status</label>
                    <select id="status-select" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                        <option value="">All Status</option>
                        <option value="valid">Valid</option>
                        <option value="expiring">Expiring Soon</option>
                        <option value="expired">Expired</option>
                        <option value="unknown">Unknown</option>
                    </select>
                </div>
                <div class="flex items-end space-x-2">
                    <button onclick="applyFilters()"
                            class="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition">
                        Apply Filters
                    </button>
                    <button onclick="resetFilters()"
                            class="w-full px-4 py-2 border border-slate-300 text-slate-700 rounded-lg hover:bg-slate-50 transition">
                        Reset
                    </button>
                </div>
            </div>
        </div>

        <!-- SSL Monitors Table -->
        <div class="bg-white rounded-xl shadow overflow-hidden">
            <div class="p-6 border-b border-slate-200 flex justify-between items-center">
                <h2 class="text-xl font-bold text-slate-800">Your SSL Certificates</h2>
                <div class="text-sm text-slate-500">
                    Last update: <span id="update-time">Just now</span>
                </div>
            </div>

            <div id="ssl-monitors-table">
                <div class="p-12 text-center">
                    <div class="inline-block animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500"></div>
                    <p class="mt-2 text-slate-500">Loading SSL monitors...</p>
                </div>
            </div>
        </div>
    </div>
</div>"##);

    // MODAL ADD SSL MONITOR
    html.push_str(r##"
    <div id="add-ssl-modal" class="modal-overlay">
        <div class="bg-white rounded-2xl w-full max-w-md max-h-[90vh] overflow-auto">
            <div class="p-6 border-b border-slate-200 sticky top-0 bg-white">
                <div class="flex justify-between items-center">
                    <h3 class="text-xl font-bold text-slate-800">➕ Add SSL Monitor</h3>
                    <button onclick="closeAddSslModal()" class="text-slate-400 hover:text-slate-600">
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>
                <p class="text-slate-600 mt-1">Monitor SSL certificate for a domain</p>
            </div>

            <form id="add-ssl-form" class="p-6">
                <div class="space-y-4">
                    <div>
                        <label class="block text-slate-700 font-medium mb-2">Domain *</label>
                        <input type="text" id="ssl-domain" name="domain"
                               class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                               placeholder="example.com" required>
                        <p class="text-sm text-slate-500 mt-1">Contoh: google.com, api.example.com</p>
                    </div>

                    <div>
                        <label class="block text-slate-700 font-medium mb-2">Port</label>
                        <input type="number" id="ssl-port" name="port" value="443"
                               class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                        <p class="text-sm text-slate-500 mt-1">Default: 443 (standard HTTPS)</p>
                    </div>

                    <div>
                        <label class="block text-slate-700 font-medium mb-2">Check Interval (hours)</label>
                        <select id="ssl-interval" name="interval" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                            <option value="6">Every 6 hours</option>
                            <option value="12">Every 12 hours</option>
                            <option value="24" selected>Every 24 hours</option>
                            <option value="48">Every 2 days</option>
                            <option value="168">Every week</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-slate-700 font-medium mb-2">Alert Before Expiry</label>
                        <div class="space-y-2">
                            <label class="flex items-center">
                                <input type="checkbox" id="alert-30" value="30" checked class="rounded border-slate-300">
                                <span class="ml-2 text-sm">30 days before</span>
                            </label>
                            <label class="flex items-center">
                                <input type="checkbox" id="alert-7" value="7" checked class="rounded border-slate-300">
                                <span class="ml-2 text-sm">7 days before</span>
                            </label>
                            <label class="flex items-center">
                                <input type="checkbox" id="alert-1" value="1" checked class="rounded border-slate-300">
                                <span class="ml-2 text-sm">1 day before</span>
                            </label>
                        </div>
                    </div>
                </div>

                <div class="flex justify-end space-x-3 mt-6 pt-4 border-t">
                    <button type="button" onclick="closeAddSslModal()"
                            class="px-6 py-2 border border-slate-300 text-slate-700 rounded-lg hover:bg-slate-50 transition">
                        Cancel
                    </button>
                    <button type="button" onclick="submitSslForm()"
                            class="px-6 py-2 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700 transition">
                        Add Monitor
                    </button>
                </div>
            </form>
        </div>
    </div>

    <script>
        let currentPage = 1;
        let pollingInterval = null;
        let sslIds = new Set();

        function openAddSslModal() {
            document.getElementById('add-ssl-modal').classList.add('active');
        }

        function closeAddSslModal() {
            document.getElementById('add-ssl-modal').classList.remove('active');
        }

        function submitSslForm() {
            const domain = document.getElementById('ssl-domain').value.trim();
            if (!domain) {
                Swal.fire({ icon: 'error', title: 'Error', text: 'Domain is required' });
                return;
            }

            const port = parseInt(document.getElementById('ssl-port').value) || 443;
            const interval = parseInt(document.getElementById('ssl-interval').value) || 24;

            // Collect alert days
            const alertDays = [];
            if (document.getElementById('alert-30').checked) alertDays.push('30');
            if (document.getElementById('alert-7').checked) alertDays.push('7');
            if (document.getElementById('alert-1').checked) alertDays.push('1');

            const monitors = [{
                domain: domain,
                port: port,
                check_interval: interval,
                alert_days: alertDays.join(',')
            }];

            fetch('/ssl-monitors', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ monitors })
            })
            .then(response => response.text())
            .then(() => {
                closeAddSslModal();
                Swal.fire({ icon: 'success', title: 'Success', text: 'SSL monitor added', timer: 2000 });
                loadSslData();
            })
            .catch(error => {
                Swal.fire({ icon: 'error', title: 'Error', text: error.message });
            });
        }

        function applyFilters() {
            currentPage = 1;
            loadSslData();
        }

        function resetFilters() {
            document.getElementById('search-input').value = '';
            document.getElementById('status-select').value = '';
            currentPage = 1;
            loadSslData();
        }

        function loadSslData() {
            const search = document.getElementById('search-input').value;
            const status = document.getElementById('status-select').value;
            let query = `?page=${currentPage}`;
            if (search) query += `&search=${encodeURIComponent(search)}`;
            if (status) query += `&status=${encodeURIComponent(status)}`;

            fetch(`/ssl-monitors/data${query}`)
                .then(response => response.text())
                .then(html => {
                    document.getElementById('ssl-monitors-table').innerHTML = html;
                    extractSslIds();
                    startPolling();
                })
                .catch(error => console.error('Error loading SSL monitors:', error));
        }

        function extractSslIds() {
            sslIds.clear();
            document.querySelectorAll('tr[id^="ssl-row-"]').forEach(row => {
                const id = row.id.replace('ssl-row-', '');
                sslIds.add(parseInt(id));
            });
        }

        function startPolling() {
            if (pollingInterval) clearInterval(pollingInterval);
            pollingInterval = setInterval(() => {
                if (sslIds.size > 0) {
                    fetchStatusUpdates();
                }
            }, 30000);
        }

        function fetchStatusUpdates() {
            if (sslIds.size === 0) return;
            const idsString = Array.from(sslIds).join(',');
            fetch(`/ssl-monitors/status-updates?ids=${idsString}`)
                .then(response => response.json())
                .then(data => updateStatuses(data))
                .catch(error => console.error('Error fetching status:', error));
        }

        function updateStatuses(statuses) {
            statuses.forEach(status => {
                const row = document.getElementById(`ssl-row-${status.id}`);
                if (row) {
                    const statusCell = row.querySelector('td:nth-child(3) span');
                    if (statusCell) {
                        statusCell.className = getStatusClass(status.status);
                        statusCell.innerHTML = getStatusHtml(status.status);
                    }
                    const daysCell = row.querySelector('td:nth-child(4)');
                    if (daysCell) daysCell.textContent = status.days_left > 0 ? status.days_left + ' days' : 'Expired';
                }
            });
        }

        function getStatusClass(status) {
            switch(status) {
                case 'valid': return 'px-3 py-1 rounded-full text-xs font-semibold bg-green-100 text-green-800';
                case 'expiring': return 'px-3 py-1 rounded-full text-xs font-semibold bg-yellow-100 text-yellow-800';
                case 'expired': return 'px-3 py-1 rounded-full text-xs font-semibold bg-red-100 text-red-800';
                default: return 'px-3 py-1 rounded-full text-xs font-semibold bg-gray-100 text-gray-800';
            }
        }

        function getStatusHtml(status) {
            switch(status) {
                case 'valid': return '✅ Valid';
                case 'expiring': return '⚠️ Expiring Soon';
                case 'expired': return '❌ Expired';
                default: return '⚪ Unknown';
            }
        }

        function loadPage(pageNum) {
            currentPage = pageNum;
            loadSslData();
        }

        function checkSslNow(sslId) {
            Swal.fire({
                title: '🔍 Checking SSL Certificate',
                text: 'Fetching SSL certificate information...',
                allowOutsideClick: false,
                didOpen: () => {
                    Swal.showLoading();

                    fetch(`/ssl-monitors/${sslId}/check`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' }
                    })
                    .then(response => response.json())
                    .then(data => {
                        if (data.success) {
                            Swal.fire({
                                icon: 'success',
                                title: '✅ SSL Certificate Updated!',
                                html: `
                                    <div class="text-left mt-2">
                                        <p><strong>Status:</strong> <span class="text-green-600">${data.status || 'Valid'}</span></p>
                                        <p><strong>Days Left:</strong> ${data.days_left || 'N/A'} days</p>
                                        <p><strong>Expiry Date:</strong> ${data.expiry_date || 'N/A'}</p>
                                        <p><strong>Issuer:</strong> ${data.issuer || 'N/A'}</p>
                                    </div>
                                `,
                                timer: 5000,
                                showConfirmButton: true
                            });
                            loadSslData();
                        } else {
                            Swal.fire({
                                icon: 'error',
                                title: '❌ Error',
                                text: data.error || 'Failed to fetch SSL certificate'
                            });
                        }
                    })
                    .catch(error => {
                        console.error('Error:', error);
                        Swal.fire({
                            icon: 'error',
                            title: 'Network Error',
                            text: 'Failed to connect to server'
                        });
                    });
                }
            });
        }

        function toggleSslStatus(sslId, currentStatus) {
            Swal.fire({
                title: 'Toggle SSL Monitor',
                text: `Are you sure you want to ${currentStatus ? 'disable' : 'enable'} this SSL monitor?`,
                icon: 'warning',
                showCancelButton: true,
                confirmButtonText: 'Yes'
            }).then((result) => {
                if (result.isConfirmed) {
                    fetch(`/ssl-monitors/${sslId}/toggle`, { method: 'POST' })
                        .then(response => {
                            if (response.ok) {
                                loadSslData();
                                Swal.fire('Changed!', `SSL monitor ${currentStatus ? 'disabled' : 'enabled'}`, 'success');
                            }
                        });
                }
            });
        }

        function deleteSslMonitor(sslId) {
            Swal.fire({
                title: 'Delete SSL Monitor',
                text: 'This action cannot be undone!',
                icon: 'error',
                showCancelButton: true,
                confirmButtonColor: '#d33',
                confirmButtonText: 'Delete'
            }).then((result) => {
                if (result.isConfirmed) {
                    fetch(`/ssl-monitors/${sslId}/delete`, { method: 'POST' })
                        .then(response => {
                            if (response.ok) {
                                sslIds.delete(sslId);
                                loadSslData();
                                Swal.fire('Deleted!', 'SSL monitor has been deleted.', 'success');
                            }
                        });
                }
            });
        }

        document.addEventListener('DOMContentLoaded', function() {
            loadSslData();
        });

        window.openAddSslModal = openAddSslModal;
        window.closeAddSslModal = closeAddSslModal;
        window.submitSslForm = submitSslForm;
        window.applyFilters = applyFilters;
        window.resetFilters = resetFilters;
        window.loadPage = loadPage;
        window.checkSslNow = checkSslNow;
        window.toggleSslStatus = toggleSslStatus;
        window.deleteSslMonitor = deleteSslMonitor;
    </script>
"##);

    html.push_str(&footer::render());
    html.push_str("</body></html>");

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

// ==================== CREATE SSL MONITOR ====================

async fn create_ssl_monitor_json(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    monitor_data: web::Json<SslMonitorJson>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let mut inserted_count = 0;
            let mut errors = Vec::new();
            let mut successful_monitors = Vec::new();

            for monitor in &monitor_data.monitors {
                let port = monitor.port.unwrap_or(443);
                let interval = monitor.check_interval.unwrap_or(24);
                let alert_days = monitor.alert_days.clone().unwrap_or_else(|| "30,7,1".to_string());

                if monitor.domain.is_empty() {
                    errors.push("Domain is required".to_string());
                    continue;
                }

                // Insert ke database
                match sqlx::query(
                    "INSERT INTO ssl_monitors
                     (user_id, domain, port, check_interval, alert_days, is_active)
                     VALUES (?, ?, ?, ?, ?, 1)"
                )
                .bind(user_id)
                .bind(&monitor.domain)
                .bind(port)
                .bind(interval)
                .bind(&alert_days)
                .execute(db_pool.get_ref())
                .await
                {
                    Ok(result) => {
                        inserted_count += 1;

                        // Ambil ID yang baru diinsert
                        let monitor_id = result.last_insert_rowid();

                        // ✅ LANGSUNG FETCH SSL CERTIFICATE
                        println!("🔍 Fetching SSL for {}:{}", monitor.domain, port);
                        match fetch_ssl_certificate(&monitor.domain, port).await {
                            Ok((expiry_date, issuer)) => {
                                println!("✅ SSL certificate fetched for {}", monitor.domain);

                                let now = chrono::Utc::now().naive_utc();
                                let days_left = (expiry_date - now).num_days();
                                let status = if expiry_date > now {
                                    if days_left < 7 {
                                        "expiring"
                                    } else {
                                        "valid"
                                    }
                                } else {
                                    "expired"
                                };

                                // Update database dengan data SSL
                                let update_result = sqlx::query!(
                                    "UPDATE ssl_monitors
                                     SET expiry_date = ?,
                                         issuer = ?,
                                         last_checked = CURRENT_TIMESTAMP,
                                         last_status = ?
                                     WHERE id = ?",
                                    expiry_date,
                                    issuer,
                                    status,
                                    monitor_id
                                )
                                .execute(db_pool.get_ref())
                                .await;

                                match update_result {
                                    Ok(_) => {
                                        successful_monitors.push(format!(
                                            "{} (SSL: valid until {})",
                                            monitor.domain,
                                            expiry_date.format("%Y-%m-%d")
                                        ));
                                    }
                                    Err(e) => {
                                        errors.push(format!(
                                            "Failed to update SSL data for '{}': {}",
                                            monitor.domain, e
                                        ));
                                    }
                                }
                            }
                            Err(e) => {
                                println!("⚠️ Could not fetch SSL for {}: {}", monitor.domain, e);
                                errors.push(format!(
                                    "Added '{}' but failed to fetch SSL certificate: {}",
                                    monitor.domain, e
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!("Failed to insert '{}': {}", monitor.domain, e));
                    }
                }
            }

            if !errors.is_empty() {
                let mut response = format!("Added {} monitor(s)\n", inserted_count);
                response.push_str(&format!("Warnings/Errors:\n{}", errors.join("\n")));
                HttpResponse::BadRequest().body(response)
            } else {
                let message = format!(
                    "✅ Added {} SSL monitor(s) with SSL data!\n{}",
                    inserted_count,
                    successful_monitors.join("\n")
                );
                HttpResponse::Ok().body(message)
            }
        }
        _ => HttpResponse::Unauthorized().body("Unauthorized"),
    }
}

// ==================== CHECK SSL NOW ====================

async fn check_ssl_now(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    monitor_id: web::Path<i64>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let id = monitor_id.into_inner();

            // Ambil monitor dari database
            let monitor = sqlx::query!(
                "SELECT domain, port, last_checked FROM ssl_monitors WHERE id = ? AND user_id = ?",
                id,
                user_id
            )
            .fetch_optional(db_pool.get_ref())
            .await;

            match monitor {
                Ok(Some(m)) => {
                    let port = m.port.unwrap_or(443);
                    println!("🔍 Checking SSL for {}:{}", m.domain, port);

                    // Fetch SSL certificate
                    match fetch_ssl_certificate(&m.domain, port).await {
                        Ok((expiry_date, issuer)) => {
                            println!("✅ SSL certificate fetched for {}", m.domain);

                            // Hitung status
                            let now = chrono::Utc::now().naive_utc();
                            let days_left = (expiry_date - now).num_days();
                            let status = if expiry_date > now {
                                if days_left < 7 {
                                    "expiring"
                                } else {
                                    "valid"
                                }
                            } else {
                                "expired"
                            };

                            // Update database
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
                                id
                            )
                            .execute(db_pool.get_ref())
                            .await
                            {
                                Ok(_) => {
                                    HttpResponse::Ok().json(serde_json::json!({
                                        "success": true,
                                        "message": "SSL certificate updated successfully",
                                        "expiry_date": expiry_date.format("%Y-%m-%d %H:%M:%S").to_string(),
                                        "issuer": issuer,
                                        "status": status,
                                        "days_left": days_left,
                                        "last_checked": Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
                                    }))
                                }
                                Err(e) => {
                                    eprintln!("❌ Database error: {}", e);
                                    HttpResponse::InternalServerError().json(serde_json::json!({
                                        "success": false,
                                        "error": "Failed to update database"
                                    }))
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ SSL fetch error for {}: {}", m.domain, e);
                            HttpResponse::BadRequest().json(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to fetch SSL certificate: {}", e)
                            }))
                        }
                    }
                }
                Ok(None) => {
                    HttpResponse::NotFound().json(serde_json::json!({
                        "success": false,
                        "error": "SSL monitor not found"
                    }))
                }
                Err(e) => {
                    eprintln!("❌ Database error: {}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "success": false,
                        "error": "Database error"
                    }))
                }
            }
        }
        _ => HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })),
    }
}

// ==================== TOGGLE SSL MONITOR ====================

async fn toggle_ssl_monitor(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    monitor_id: web::Path<i64>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let id = monitor_id.into_inner();
            match sqlx::query!(
                "SELECT is_active FROM ssl_monitors WHERE id = ? AND user_id = ?",
                id,
                user_id
            )
            .fetch_optional(db_pool.get_ref())
            .await
            {
                Ok(Some(monitor)) => {
                    let current_status = monitor.is_active.unwrap_or(true);
                    let new_status = !current_status;

                    match sqlx::query!(
                        "UPDATE ssl_monitors SET is_active = ? WHERE id = ?",
                        new_status,
                        id
                    )
                    .execute(db_pool.get_ref())
                    .await
                    {
                        Ok(_) => HttpResponse::Ok().finish(),
                        Err(e) => {
                            eprintln!("❌ Failed to toggle SSL monitor: {}", e);
                            HttpResponse::InternalServerError().finish()
                        }
                    }
                }
                _ => HttpResponse::NotFound().finish(),
            }
        }
        _ => HttpResponse::Unauthorized().finish(),
    }
}

// ==================== DELETE SSL MONITOR ====================

async fn delete_ssl_monitor(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    monitor_id: web::Path<i64>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let id = monitor_id.into_inner();
            match sqlx::query!(
                "DELETE FROM ssl_monitors WHERE id = ? AND user_id = ?",
                id,
                user_id
            )
            .execute(db_pool.get_ref())
            .await
            {
                Ok(_) => HttpResponse::Ok().finish(),
                Err(e) => {
                    eprintln!("❌ Failed to delete SSL monitor: {}", e);
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        _ => HttpResponse::Unauthorized().finish(),
    }
}

// ==================== SSL MONITORS DATA ====================

async fn ssl_monitors_data(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    query: web::Query<SslMonitorQuery>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let page = query.page.unwrap_or(1);
            let limit = 10;
            let offset = (page - 1) * limit;
            let search = query.search.clone().unwrap_or_default();
            let status_filter = query.status.clone().unwrap_or_default();

            let mut monitors = Vec::new();

            // Get SSL monitors
            let rows = sqlx::query(
                "SELECT id, domain, port, expiry_date, issuer, is_active, last_checked
                 FROM ssl_monitors WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?"
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(db_pool.get_ref())
            .await
            .unwrap_or(vec![]);

            for row in rows {
                let id: i64 = row.get(0);
                let domain: String = row.get(1);
                let port: i64 = row.get(2);
                let expiry: Option<NaiveDateTime> = row.get(3);
                let issuer: Option<String> = row.get(4);
                let is_active: bool = row.get(5);
                let last_checked: Option<NaiveDateTime> = row.get(6);

                let (status, days_left) = calculate_ssl_status(&expiry);

                // Apply status filter
                if !status_filter.is_empty() && status != status_filter {
                    continue;
                }

                // Apply search filter
                if !search.is_empty() && !domain.contains(&search) {
                    continue;
                }

                monitors.push(SslMonitorRow {
                    id,
                    domain,
                    port,
                    status: status.clone(),
                    status_class: get_status_class(&status),
                    days_left,
                    expiry_date: expiry.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default(),
                    issuer: issuer.unwrap_or_else(|| "Unknown".to_string()),
                    is_active,
                    last_checked: last_checked.map(|d| format_last_checked(d)),
                });
            }

            let total = monitors.len() as i64;
            let total_pages = if total > 0 { (total as f64 / limit as f64).ceil() as i64 } else { 1 };

            let table_html = render_ssl_table(&monitors, page, total_pages, total);
            HttpResponse::Ok().content_type("text/html").body(table_html)
        }
        _ => HttpResponse::Unauthorized().finish(),
    }
}

// ==================== HELPER FUNCTIONS ====================

fn calculate_ssl_status(expiry: &Option<NaiveDateTime>) -> (String, i64) {
    if let Some(expiry_date) = expiry {
        let now = Utc::now().naive_utc();
        let days_left = (*expiry_date - now).num_days();

        if days_left < 0 {
            ("expired".to_string(), days_left)
        } else if days_left < 7 {
            ("expiring".to_string(), days_left)
        } else {
            ("valid".to_string(), days_left)
        }
    } else {
        ("unknown".to_string(), 0)
    }
}

fn get_status_class(status: &str) -> String {
    match status {
        "valid" => "badge-valid".to_string(),
        "expiring" => "badge-expiring".to_string(),
        "expired" => "badge-expired".to_string(),
        _ => "badge-unknown".to_string(),
    }
}

fn get_status_text(status: &str) -> String {
    match status {
        "valid" => "✅ Valid".to_string(),
        "expiring" => "⚠️ Expiring Soon".to_string(),
        "expired" => "❌ Expired".to_string(),
        _ => "⚪ Unknown".to_string(),
    }
}

// ==================== STATUS UPDATES ====================

async fn ssl_status_updates(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    query: web::Query<StatusQuery>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let ids_vec: Vec<i64> = query.ids
                .split(',')
                .filter_map(|s| s.parse().ok())
                .collect();

            let mut statuses = Vec::new();
            for id in ids_vec {
                let row = sqlx::query!(
                    "SELECT domain, expiry_date FROM ssl_monitors WHERE id = ? AND user_id = ?",
                    id,
                    user_id
                )
                .fetch_optional(db_pool.get_ref())
                .await
                .unwrap_or(None);

                if let Some(r) = row {
                    let expiry_string = r.expiry_date.map(|d| d.to_string());
                    let (status, days_left) = calculate_ssl_status(&r.expiry_date);

                    statuses.push(SslStatus {
                        id,
                        domain: r.domain,
                        status,
                        days_left,
                        expiry_date: expiry_string,
                    });
                }
            }

            HttpResponse::Ok().json(statuses)
        }
        _ => HttpResponse::Unauthorized().finish(),
    }
}

// ==================== RENDER TABLE ====================

fn render_ssl_table(monitors: &[SslMonitorRow], current_page: i64, total_pages: i64, total_records: i64) -> String {
    let mut html = String::new();

    if monitors.is_empty() {
        return r##"<div class="p-12 text-center"><h3 class="text-lg font-medium">No SSL monitors found</h3></div>"##.to_string();
    }

    html.push_str(r#"<div class="overflow-x-auto"><table class="w-full"><thead><tr class="bg-slate-50">"#);
    html.push_str(r#"<th class="text-left py-4 px-6">Domain</th><th class="text-left py-4 px-6">Port</th>"#);
    html.push_str(r#"<th class="text-left py-4 px-6">Status</th><th class="text-left py-4 px-6">Days Left</th>"#);
    html.push_str(r#"<th class="text-left py-4 px-6">Expiry Date</th><th class="text-left py-4 px-6">Issuer</th>"#);
    html.push_str(r#"<th class="text-left py-4 px-6">Last Checked</th><th class="text-left py-4 px-6">Active</th><th class="text-left py-4 px-6">Actions</th> </tr></thead><tbody>"#);

    for m in monitors {
        let days_display = if m.days_left < 0 {
            "Expired".to_string()
        } else {
            format!("{} days", m.days_left)
        };

        html.push_str(&format!(
            r#"<tr id="ssl-row-{}" class="border-b hover:bg-slate-50">
                <td class="py-4 px-6 font-medium">{}</td>
                <td class="py-4 px-6">{}</td>
                <td class="py-4 px-6"><span class="{}">{}</span></td>
                <td class="py-4 px-6">{}</td>
                <td class="py-4 px-6 text-sm">{}</td>
                <td class="py-4 px-6 text-sm">{}</td>
                <td class="py-4 px-6 text-sm">{}</td>
                <td class="py-4 px-6">
                    <label class="switch">
                        <input type="checkbox" {} onchange="toggleSslStatus({}, {})">
                        <span class="slider round"></span>
                    </label>
                </td>
                <td class="py-4 px-6">
                    <button onclick="checkSslNow({})"
                            class="p-1 text-blue-600 hover:text-blue-800 mr-2 btn-check"
                            title="Check SSL Now">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                    </button>
                    <button onclick="deleteSslMonitor({})"
                            class="p-1 text-red-600 hover:text-red-800"
                            title="Delete">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                  d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                        </svg>
                    </button>
                </td>
             </tr>"#,
            m.id, m.domain, m.port,
            if m.status == "valid" { "px-3 py-1 rounded-full text-xs font-semibold bg-green-100 text-green-800" }
            else if m.status == "expiring" { "px-3 py-1 rounded-full text-xs font-semibold bg-yellow-100 text-yellow-800" }
            else if m.status == "expired" { "px-3 py-1 rounded-full text-xs font-semibold bg-red-100 text-red-800" }
            else { "px-3 py-1 rounded-full text-xs font-semibold bg-gray-100 text-gray-800" },
            get_status_text(&m.status),
            days_display, m.expiry_date, m.issuer,
            m.last_checked.as_deref().unwrap_or("Never"),
            if m.is_active { "checked" } else { "" }, m.id, if !m.is_active { "true" } else { "false" },
            m.id,  // untuk checkSslNow
            m.id   // untuk deleteSslMonitor
        ));
    }

    html.push_str(r#"</tbody></table></div>"#);

    if total_pages > 1 {
        html.push_str("<div class=\"p-4 border-t\"><div class=\"flex justify-between items-center\">");
        html.push_str(&format!("Showing {} to {} of {} monitors",
            ((current_page - 1) * 10 + 1).min(total_records),
            (current_page * 10).min(total_records),
            total_records
        ));
        html.push_str("<div class=\"flex space-x-1\">");

        if current_page > 1 {
            html.push_str(&format!("<button onclick=\"loadPage({})\" class=\"px-3 py-1 border rounded hover:bg-slate-50\">«</button>", current_page - 1));
        }

        let start = (current_page - 2).max(1);
        let end = (current_page + 2).min(total_pages);
        for p in start..=end {
            if p == current_page {
                html.push_str(&format!("<button class=\"px-3 py-1 bg-blue-500 text-white rounded\">{}</button>", p));
            } else {
                html.push_str(&format!("<button onclick=\"loadPage({})\" class=\"px-3 py-1 border rounded hover:bg-slate-50\">{}</button>", p, p));
            }
        }

        if current_page < total_pages {
            html.push_str(&format!("<button onclick=\"loadPage({})\" class=\"px-3 py-1 border rounded hover:bg-slate-50\">»</button>", current_page + 1));
        }

        html.push_str("</div></div></div>");
    }

    html
}
