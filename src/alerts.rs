use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::{SqlitePool, Row};
use serde_json::json;

// INCLUDE SEPERTI DASHBOARD
use crate::navbar;
use crate::sidebar;
use crate::footer;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/alerts")
            .route("", web::get().to(alerts_page))
            .route("/data", web::get().to(alerts_data))
            .route("/{id}", web::get().to(get_alert))
            .route("/{id}/acknowledge", web::post().to(acknowledge_alert))
            .route("/{id}/resolve", web::post().to(resolve_alert))
            .route("/bulk-acknowledge", web::post().to(bulk_acknowledge))
            .route("/bulk-resolve", web::post().to(bulk_resolve))
            .route("/bulk-delete", web::post().to(bulk_delete))
            .route("/mark-all-read", web::post().to(mark_all_read))
            .route("/acknowledge-all", web::post().to(acknowledge_all))
            // ========== TAMBAHKAN ROUTES UNTUK SSL ALERTS ==========
            .route("/ssl/{id}", web::get().to(get_ssl_alert))
            .route("/ssl/{id}/acknowledge", web::post().to(acknowledge_ssl_alert))
            .route("/ssl/{id}/resolve", web::post().to(resolve_ssl_alert))
    );
}

// ==================== SSL ALERT HANDLERS ====================

async fn get_ssl_alert(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let alert_id = path.into_inner();

    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let alert = sqlx::query(
                        "SELECT
                            a.id,
                            a.title,
                            a.message,
                            a.severity,
                            a.status,
                            a.triggered_at,
                            a.resolved_at,
                            a.acknowledged_at,
                            s.domain as monitor_name
                         FROM ssl_alerts a
                         LEFT JOIN ssl_monitors s ON a.ssl_monitor_id = s.id
                         WHERE a.id = ? AND a.user_id = ?"
                    )
                    .bind(alert_id)
                    .bind(user_id)
                    .fetch_optional(db_pool.get_ref())
                    .await;

                    match alert {
                        Ok(Some(row)) => {
                            let id: i64 = row.get("id");
                            let title: String = row.get("title");
                            let message: String = row.get("message");
                            let severity: String = row.get("severity");
                            let status: String = row.get("status");
                            let triggered_at: String = row.get("triggered_at");
                            let resolved_at: Option<String> = row.get("resolved_at");
                            let acknowledged_at: Option<String> = row.get("acknowledged_at");
                            let monitor_name: String = row.get("monitor_name");

                            HttpResponse::Ok().json(json!({
                                "id": id,
                                "title": title,
                                "message": message,
                                "severity": severity,
                                "status": status,
                                "triggered_at": triggered_at,
                                "resolved_at": resolved_at,
                                "acknowledged_at": acknowledged_at,
                                "monitor_name": monitor_name,
                                "domain": monitor_name,
                                "alert_type": "ssl"
                            }))
                        }
                        Ok(None) => HttpResponse::NotFound().json(json!({
                            "status": "error",
                            "message": "SSL Alert not found"
                        })),
                        Err(e) => {
                            eprintln!("Database error: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "status": "error",
                                "message": "Database error"
                            }))
                        }
                    }
                }
                Ok(None) => HttpResponse::Unauthorized().json(json!({
                    "status": "error",
                    "message": "User not found"
                })),
                Err(e) => {
                    eprintln!("Database error: {:?}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "status": "error",
                        "message": "Database error"
                    }))
                }
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn acknowledge_ssl_alert(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let alert_id = path.into_inner();

    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let result = sqlx::query(
                        "UPDATE ssl_alerts
                         SET status = 'acknowledged', acknowledged_at = datetime('now')
                         WHERE id = ? AND user_id = ? AND status = 'active'"
                    )
                    .bind(alert_id)
                    .bind(user_id)
                    .execute(db_pool.get_ref())
                    .await;

                    match result {
                        Ok(rows) => {
                            if rows.rows_affected() > 0 {
                                HttpResponse::Ok().json(json!({
                                    "status": "success",
                                    "message": format!("SSL Alert {} acknowledged", alert_id)
                                }))
                            } else {
                                HttpResponse::NotFound().json(json!({
                                    "status": "error",
                                    "message": "SSL Alert not found or already acknowledged"
                                }))
                            }
                        }
                        Err(e) => {
                            eprintln!("Database error: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "status": "error",
                                "message": "Database error"
                            }))
                        }
                    }
                }
                _ => HttpResponse::Unauthorized().json(json!({
                    "status": "error",
                    "message": "Authentication failed"
                }))
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn resolve_ssl_alert(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let alert_id = path.into_inner();

    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let result = sqlx::query(
                        "UPDATE ssl_alerts
                         SET status = 'resolved', resolved_at = datetime('now')
                         WHERE id = ? AND user_id = ? AND status != 'resolved'"
                    )
                    .bind(alert_id)
                    .bind(user_id)
                    .execute(db_pool.get_ref())
                    .await;

                    match result {
                        Ok(rows) => {
                            if rows.rows_affected() > 0 {
                                HttpResponse::Ok().json(json!({
                                    "status": "success",
                                    "message": format!("SSL Alert {} resolved", alert_id)
                                }))
                            } else {
                                HttpResponse::NotFound().json(json!({
                                    "status": "error",
                                    "message": "SSL Alert not found or already resolved"
                                }))
                            }
                        }
                        Err(e) => {
                            eprintln!("Database error: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "status": "error",
                                "message": "Database error"
                            }))
                        }
                    }
                }
                _ => HttpResponse::Unauthorized().json(json!({
                    "status": "error",
                    "message": "Authentication failed"
                }))
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn alerts_page(session: Session, db_pool: web::Data<SqlitePool>) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => render_alerts(&email, &db_pool).await,
        _ => HttpResponse::Found()
            .append_header(("Location", "/"))
            .finish(),
    }
}

async fn render_alerts(email: &str, db_pool: &SqlitePool) -> HttpResponse {
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

    // HTML HEAD dan CSS
    html.push_str(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Alerts - Basic Monitoring</title>
    <link href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/sweetalert2@11"></script>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
        body { font-family: 'Inter', sans-serif; background-color: #f8fafc; }
        .sidebar-bg { background: linear-gradient(180deg, #1e293b 0%, #0f172a 100%); }
        .stat-card { transition: transform 0.2s ease; }
        .stat-card:hover { transform: translateY(-2px); }
        .status-badge { padding: 2px 8px; border-radius: 9999px; font-size: 0.75rem; font-weight: 600; }
        .status-active { background-color: #fee2e2; color: #991b1b; }
        .status-acknowledged { background-color: #fef3c7; color: #92400e; }
        .status-resolved { background-color: #d1fae5; color: #065f46; }
        .severity-critical { background-color: #fef2f2; border-left: 4px solid #dc2626; }
        .severity-warning { background-color: #fffbeb; border-left: 4px solid #f59e0b; }
        .severity-info { background-color: #eff6ff; border-left: 4px solid #3b82f6; }
        .alert-row:hover { background-color: #f8fafc; }
        .alert-checkbox { border-radius: 4px; border: 2px solid #cbd5e1; }
        .alert-checkbox:checked { background-color: #3b82f6; border-color: #3b82f6; }
        .loading-spinner { display: inline-block; width: 20px; height: 20px; border: 2px solid #e5e7eb; border-top-color: #3b82f6; border-radius: 50%; animation: spin 1s linear infinite; }
        @keyframes spin { to { transform: rotate(360deg); } }
        .filter-panel { transition: all 0.3s ease; }
        .pagination-btn { padding: 6px 12px; border: 1px solid #d1d5db; background-color: white; color: #374151; border-radius: 6px; cursor: pointer; transition: all 0.2s ease; }
        .pagination-btn:hover:not(:disabled) { background-color: #f3f4f6; border-color: #9ca3af; }
        .pagination-btn.active { background-color: #3b82f6; color: white; border-color: #3b82f6; }
        .pagination-btn:disabled { opacity: 0.5; cursor: not-allowed; }
        .bulk-actions-bar { background-color: #f8fafc; border-top: 1px solid #e5e7eb; transition: all 0.3s ease; }
    </style>
</head>
<body>"##);

    // INCLUDE NAVBAR
    html.push_str(&navbar::render(email));

    // WRAPPER
    html.push_str(r#"<div class="flex">"#);

    // INCLUDE SIDEBAR
    html.push_str(&sidebar::render());

    // KONTEN UTAMA ALERTS
    html.push_str(r##"
        <!-- Main Content -->
        <div class="flex-1 p-6">
            <!-- Header -->
            <div class="flex justify-between items-center mb-6">
                <div>
                    <h1 class="text-2xl font-bold text-gray-800">Alerts</h1>
                    <p class="text-gray-600">Monitor and manage your website and SSL alerts</p>
                </div>
                <div class="flex space-x-3">
                    <button id="markAllRead" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition">
                        Mark All as Read
                    </button>
                    <button id="acknowledgeAll" class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition">
                        Acknowledge All
                    </button>
                </div>
            </div>

            <!-- Filter Bar -->
            <div class="bg-white rounded-lg shadow p-4 mb-6 filter-panel">
                <div class="flex flex-wrap gap-4 items-center">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Type</label>
                        <select id="typeFilter" class="w-40 border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500">
                            <option value="all">All Types</option>
                            <option value="website">Website</option>
                            <option value="ssl">SSL Certificate</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Status</label>
                        <select id="statusFilter" class="w-40 border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500">
                            <option value="all">All Status</option>
                            <option value="active">Active</option>
                            <option value="resolved">Resolved</option>
                            <option value="acknowledged">Acknowledged</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Severity</label>
                        <select id="severityFilter" class="w-40 border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500">
                            <option value="all">All Severities</option>
                            <option value="critical">Critical</option>
                            <option value="warning">Warning</option>
                            <option value="info">Info</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Date Range</label>
                        <select id="dateFilter" class="w-40 border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500">
                            <option value="24h">Last 24 Hours</option>
                            <option value="7d">Last 7 Days</option>
                            <option value="30d">Last 30 Days</option>
                            <option value="all">All Time</option>
                        </select>
                    </div>

                    <div class="flex-1">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Search</label>
                        <input type="text" id="searchAlerts" placeholder="Search alerts..."
                               class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500">
                    </div>

                    <div class="mt-6">
                        <button id="applyFilters" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition">
                            Apply Filters
                        </button>
                        <button id="clearFilters" class="px-4 py-2 bg-gray-300 text-gray-700 rounded-lg hover:bg-gray-400 ml-2 transition">
                            Clear
                        </button>
                    </div>
                </div>
            </div>

            <!-- Stats Cards -->
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
                <div class="stat-card bg-white rounded-lg shadow p-4">
                    <div class="flex items-center">
                        <div class="w-3 h-3 bg-red-500 rounded-full mr-2"></div>
                        <span class="text-sm text-gray-600">Active Alerts</span>
                    </div>
                    <p class="text-2xl font-bold mt-2" id="active-alerts-count">0</p>
                </div>

                <div class="stat-card bg-white rounded-lg shadow p-4">
                    <div class="flex items-center">
                        <div class="w-3 h-3 bg-yellow-500 rounded-full mr-2"></div>
                        <span class="text-sm text-gray-600">Warning</span>
                    </div>
                    <p class="text-2xl font-bold mt-2" id="warning-alerts-count">0</p>
                </div>

                <div class="stat-card bg-white rounded-lg shadow p-4">
                    <div class="flex items-center">
                        <div class="w-3 h-3 bg-green-500 rounded-full mr-2"></div>
                        <span class="text-sm text-gray-600">Resolved (24h)</span>
                    </div>
                    <p class="text-2xl font-bold mt-2" id="resolved-alerts-count">0</p>
                </div>

                <div class="stat-card bg-white rounded-lg shadow p-4">
                    <div class="flex items-center">
                        <div class="w-3 h-3 bg-blue-500 rounded-full mr-2"></div>
                        <span class="text-sm text-gray-600">Avg Response Time</span>
                    </div>
                    <p class="text-2xl font-bold mt-2" id="avg-response-time">0ms</p>
                </div>
            </div>

            <!-- Alerts Table -->
            <div class="bg-white rounded-lg shadow overflow-hidden">
                <div id="alerts-loading" class="p-12 text-center">
                    <div class="loading-spinner mx-auto"></div>
                    <p class="mt-2 text-slate-500">Loading alerts...</p>
                </div>

                <div id="alerts-container" class="hidden">
                    <div class="overflow-x-auto">
                        <table class="min-w-full divide-y divide-gray-200">
                            <thead class="bg-gray-50">
                                <tr>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                        <input type="checkbox" id="selectAll" class="alert-checkbox">
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                        Type
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                        Status
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                        Severity
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                        Monitor
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                        Message
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                        Triggered
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                        Duration
                                    </th>
                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                        Actions
                                    </th>
                                </tr>
                            </thead>
                            <tbody id="alerts-body" class="bg-white divide-y divide-gray-200">
                                <!-- Alerts will be loaded here via JavaScript -->
                            </tbody>
                        </table>
                    </div>

                    <!-- Pagination -->
                    <div class="px-6 py-4 border-t border-gray-200">
                        <div class="flex items-center justify-between">
                            <div>
                                <p class="text-sm text-gray-700" id="pagination-info">
                                    Loading...
                                </p>
                            </div>
                            <div class="flex space-x-2" id="pagination-controls">
                                <!-- Pagination buttons will be loaded here -->
                            </div>
                        </div>
                    </div>
                </div>

                <!-- No Alerts Message -->
                <div id="no-alerts" class="hidden p-12 text-center">
                    <div class="text-gray-400 text-4xl mb-4">🔔</div>
                    <h3 class="text-lg font-semibold text-gray-700 mb-2">No Alerts Found</h3>
                    <p class="text-gray-500">When alerts are triggered, they will appear here.</p>
                </div>
            </div>

            <!-- Bulk Actions Bar (hidden by default) -->
            <div id="bulk-actions-bar" class="hidden mt-6 p-4 bg-gray-50 rounded-lg shadow bulk-actions-bar">
                <div class="flex items-center justify-between">
                    <div>
                        <span id="selectedCount" class="font-medium">0 alerts selected</span>
                    </div>
                    <div class="flex space-x-2">
                        <button id="bulkAcknowledge" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition disabled:opacity-50 disabled:cursor-not-allowed" disabled>
                            Acknowledge Selected
                        </button>
                        <button id="bulkResolve" class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition disabled:opacity-50 disabled:cursor-not-allowed" disabled>
                            Resolve Selected
                        </button>
                        <button id="bulkDelete" class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition disabled:opacity-50 disabled:cursor-not-allowed" disabled>
                            Delete Selected
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
    "##);

    // INCLUDE FOOTER
    html.push_str(&footer::render());

    // ✅ JAVASCRIPT LENGKAP DENGAN DUKUNGAN SSL ALERTS
    html.push_str(r##"
    <script>
        let currentPage = 1;
        let totalPages = 1;
        let selectedAlerts = new Set();
        let currentFilters = {
            type: 'all',
            status: 'all',
            severity: 'all',
            dateRange: '24h',
            search: ''
        };

        document.addEventListener('DOMContentLoaded', function() {
            console.log('Alerts page loaded');
            loadAlerts(1);
            setupEventListeners();
        });

        function setupEventListeners() {
            document.getElementById('typeFilter').addEventListener('change', function() {
                currentFilters.type = this.value;
                loadAlerts(1);
            });

            document.getElementById('statusFilter').addEventListener('change', function() {
                currentFilters.status = this.value;
                loadAlerts(1);
            });

            document.getElementById('severityFilter').addEventListener('change', function() {
                currentFilters.severity = this.value;
                loadAlerts(1);
            });

            document.getElementById('dateFilter').addEventListener('change', function() {
                currentFilters.dateRange = this.value;
                loadAlerts(1);
            });

            document.getElementById('searchAlerts').addEventListener('input', function() {
                currentFilters.search = this.value;
                clearTimeout(window.searchTimeout);
                window.searchTimeout = setTimeout(() => loadAlerts(1), 300);
            });

            document.getElementById('applyFilters').addEventListener('click', () => loadAlerts(1));
            document.getElementById('clearFilters').addEventListener('click', clearFilters);

            document.getElementById('markAllRead').addEventListener('click', markAllRead);
            document.getElementById('acknowledgeAll').addEventListener('click', acknowledgeAll);
            document.getElementById('bulkAcknowledge').addEventListener('click', bulkAcknowledge);
            document.getElementById('bulkResolve').addEventListener('click', bulkResolve);
            document.getElementById('bulkDelete').addEventListener('click', bulkDelete);

            document.getElementById('selectAll').addEventListener('change', toggleSelectAll);
        }

        function loadAlerts(page = 1) {
            currentPage = page;

            document.getElementById('alerts-loading').classList.remove('hidden');
            document.getElementById('alerts-container').classList.add('hidden');
            document.getElementById('no-alerts').classList.add('hidden');

            const params = new URLSearchParams({
                page: page,
                type: currentFilters.type,
                status: currentFilters.status,
                severity: currentFilters.severity,
                date_range: currentFilters.dateRange,
                search: currentFilters.search
            });

            fetch(`/alerts/data?${params.toString()}`)
                .then(response => {
                    if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`);
                    return response.json();
                })
                .then(data => {
                    updateAlertsUI(data);
                    updateStats(data.stats || {});
                })
                .catch(error => {
                    console.error('Error loading alerts:', error);
                    showError('Failed to load alerts. Please try again.');
                    document.getElementById('alerts-loading').classList.add('hidden');
                    document.getElementById('no-alerts').classList.remove('hidden');
                });
        }

        function updateAlertsUI(data) {
            const alertsBody = document.getElementById('alerts-body');
            const alertsContainer = document.getElementById('alerts-container');
            const loadingElement = document.getElementById('alerts-loading');
            const noAlertsElement = document.getElementById('no-alerts');

            loadingElement.classList.add('hidden');

            if (!data.alerts || data.alerts.length === 0) {
                alertsContainer.classList.add('hidden');
                noAlertsElement.classList.remove('hidden');
                return;
            }

            alertsContainer.classList.remove('hidden');
            noAlertsElement.classList.add('hidden');

            alertsBody.innerHTML = '';
            selectedAlerts.clear();
            updateSelectedCount();

            data.alerts.forEach(alert => {
                const row = createAlertRow(alert);
                alertsBody.appendChild(row);
            });

            updatePagination(data.pagination);

            const paginationInfo = document.getElementById('pagination-info');
            const start = (data.pagination.current_page - 1) * data.pagination.per_page + 1;
            const end = Math.min(start + data.pagination.per_page - 1, data.pagination.total);
            paginationInfo.textContent = `Showing ${start} to ${end} of ${data.pagination.total} alerts`;
        }

        function createAlertRow(alert) {
            const row = document.createElement('tr');
            row.className = `alert-row ${getSeverityRowClass(alert.severity)}`;
            row.setAttribute('data-alert-id', alert.id);

            row.innerHTML = `
                <td class="px-6 py-4 whitespace-nowrap">
                    <input type="checkbox" class="alert-checkbox" data-alert-id="${alert.id}">
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                    ${getTypeBadge(alert.alert_type)}
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                    ${getStatusBadge(alert.status)}
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                    ${getSeverityIndicator(alert.severity)}
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                    <div class="flex items-center">
                        <span class="mr-2">${getMonitorIcon(alert.monitor_type)}</span>
                        <span>${alert.monitor_name}</span>
                    </div>
                </td>
                <td class="px-6 py-4">
                    <div class="max-w-xs">
                        <p class="font-medium">${alert.title}</p>
                        <p class="text-sm text-gray-500 truncate">${alert.message}</p>
                    </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                    ${getTimeInfo(alert)}
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                    <span class="font-medium">${getDuration(alert)}</span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                    <div class="flex space-x-2">
                        ${getActionsHtml(alert)}
                    </div>
                </td>
            `;

            const checkbox = row.querySelector('.alert-checkbox');
            checkbox.addEventListener('change', function() {
                const alertId = this.getAttribute('data-alert-id');
                if (this.checked) selectedAlerts.add(alertId);
                else selectedAlerts.delete(alertId);
                updateSelectedCount();
            });

            setTimeout(() => {
                const acknowledgeBtn = row.querySelector('.acknowledge-btn');
                const resolveBtn = row.querySelector('.resolve-btn');
                const detailsBtn = row.querySelector('.details-btn');

                if (acknowledgeBtn) {
                    acknowledgeBtn.addEventListener('click', () => acknowledgeAlert(alert.id, alert.alert_type));
                }
                if (resolveBtn) {
                    resolveBtn.addEventListener('click', () => resolveAlert(alert.id, alert.alert_type));
                }
                if (detailsBtn) {
                    detailsBtn.addEventListener('click', () => showAlertDetails(alert.id, alert.alert_type));
                }
            }, 0);

            return row;
        }

        function getTypeBadge(type) {
            const badges = {
                website: '<span class="status-badge bg-blue-100 text-blue-800">🌐 Website</span>',
                ssl: '<span class="status-badge bg-purple-100 text-purple-800">🔒 SSL</span>'
            };
            return badges[type] || '<span class="status-badge bg-gray-100 text-gray-800">Unknown</span>';
        }

        function getStatusBadge(status) {
            const badges = {
                active: '<span class="status-badge status-active">Active</span>',
                acknowledged: '<span class="status-badge status-acknowledged">Acknowledged</span>',
                resolved: '<span class="status-badge status-resolved">Resolved</span>'
            };
            return badges[status] || '<span class="status-badge bg-gray-100 text-gray-800">Unknown</span>';
        }

        function getSeverityIndicator(severity) {
            const indicators = {
                critical: '<div class="flex items-center"><div class="w-3 h-3 bg-red-500 rounded-full mr-2"></div><span class="font-semibold text-red-700">Critical</span></div>',
                warning: '<div class="flex items-center"><div class="w-3 h-3 bg-yellow-500 rounded-full mr-2"></div><span class="font-semibold text-yellow-700">Warning</span></div>',
                info: '<div class="flex items-center"><div class="w-3 h-3 bg-blue-500 rounded-full mr-2"></div><span class="font-semibold text-blue-700">Info</span></div>'
            };
            return indicators[severity] || '<div class="flex items-center"><div class="w-3 h-3 bg-gray-500 rounded-full mr-2"></div><span class="font-semibold text-gray-700">Unknown</span></div>';
        }

        function getSeverityRowClass(severity) {
            switch(severity) {
                case 'critical': return 'severity-critical';
                case 'warning': return 'severity-warning';
                case 'info': return 'severity-info';
                default: return '';
            }
        }

        function getMonitorIcon(type) {
            const icons = {
                website: '🌐',
                ssl: '🔒',
                http: '🌐',
                https: '🔒',
                ping: '📡',
                default: '📊'
            };
            return icons[type] || icons.default;
        }

        function getTimeInfo(alert) {
            const triggered = new Date(alert.triggered_at + 'Z');
            const now = new Date();
            const diffMs = now - triggered;
            const diffMins = Math.floor(diffMs / 60000);
            const diffHours = Math.floor(diffMs / 3600000);
            const diffDays = Math.floor(diffMs / 86400000);

            let timeAgo;
            if (diffDays > 0) timeAgo = `${diffDays} day${diffDays > 1 ? 's' : ''} ago`;
            else if (diffHours > 0) timeAgo = `${diffHours} hour${diffHours > 1 ? 's' : ''} ago`;
            else if (diffMins > 0) timeAgo = `${diffMins} minute${diffMins > 1 ? 's' : ''} ago`;
            else timeAgo = 'Just now';

            return `<div><p class="font-medium">${timeAgo}</p><p class="text-sm text-gray-500">${triggered.toLocaleString()}</p></div>`;
        }

        function getDuration(alert) {
            if (!alert.triggered_at) return 'N/A';

            const triggered = new Date(alert.triggered_at + 'Z');
            const resolved = alert.resolved_at ? new Date(alert.resolved_at + 'Z') : new Date();
            const diffMs = resolved - triggered;

            const diffMins = Math.floor(diffMs / 60000);
            const diffHours = Math.floor(diffMs / 3600000);

            if (diffHours > 0) return `${diffHours}h ${diffMins % 60}m`;
            else if (diffMins > 0) return `${diffMins}m`;
            else return '< 1m';
        }

        function getActionsHtml(alert) {
            let buttons = '';

            if (alert.status === 'active') {
                buttons += `
                    <button class="acknowledge-btn px-3 py-1 bg-blue-600 text-white text-xs rounded hover:bg-blue-700 transition">
                        Acknowledge
                    </button>
                    <button class="resolve-btn px-3 py-1 bg-green-600 text-white text-xs rounded hover:bg-green-700 transition ml-2">
                        Resolve
                    </button>
                `;
            } else if (alert.status === 'acknowledged') {
                buttons += `
                    <button class="resolve-btn px-3 py-1 bg-green-600 text-white text-xs rounded hover:bg-green-700 transition">
                        Resolve
                    </button>
                `;
            }

            buttons += `
                <button class="details-btn px-3 py-1 bg-gray-300 text-gray-700 text-xs rounded hover:bg-gray-400 transition ml-2">
                    Details
                </button>
            `;

            return buttons;
        }

        function updatePagination(pagination) {
            const controls = document.getElementById('pagination-controls');
            controls.innerHTML = '';

            totalPages = pagination.total_pages;

            const prevButton = document.createElement('button');
            prevButton.className = 'pagination-btn';
            prevButton.innerHTML = 'Previous';
            prevButton.disabled = currentPage === 1;
            prevButton.addEventListener('click', () => loadAlerts(currentPage - 1));
            controls.appendChild(prevButton);

            const maxVisible = 5;
            let startPage = Math.max(1, currentPage - Math.floor(maxVisible / 2));
            let endPage = Math.min(totalPages, startPage + maxVisible - 1);

            for (let i = startPage; i <= endPage; i++) {
                const pageButton = document.createElement('button');
                pageButton.className = `pagination-btn ${i === currentPage ? 'active' : ''}`;
                pageButton.textContent = i;
                pageButton.addEventListener('click', () => loadAlerts(i));
                controls.appendChild(pageButton);
            }

            const nextButton = document.createElement('button');
            nextButton.className = 'pagination-btn';
            nextButton.innerHTML = 'Next';
            nextButton.disabled = currentPage === totalPages;
            nextButton.addEventListener('click', () => loadAlerts(currentPage + 1));
            controls.appendChild(nextButton);
        }

        function updateStats(stats) {
            document.getElementById('active-alerts-count').textContent = stats.active || 0;
            document.getElementById('warning-alerts-count').textContent = stats.warning || 0;
            document.getElementById('resolved-alerts-count').textContent = stats.resolved_24h || 0;
            document.getElementById('avg-response-time').textContent = `${stats.avg_response_time || 0}ms`;
        }

        function updateSelectedCount() {
            const count = selectedAlerts.size;
            document.getElementById('selectedCount').textContent = `${count} alert${count !== 1 ? 's' : ''} selected`;

            const bulkActionsBar = document.getElementById('bulk-actions-bar');
            if (count > 0) bulkActionsBar.classList.remove('hidden');
            else bulkActionsBar.classList.add('hidden');

            document.getElementById('bulkAcknowledge').disabled = count === 0;
            document.getElementById('bulkResolve').disabled = count === 0;
            document.getElementById('bulkDelete').disabled = count === 0;
        }

        function toggleSelectAll(e) {
            const checkboxes = document.querySelectorAll('.alert-checkbox');
            checkboxes.forEach(cb => {
                cb.checked = e.target.checked;
                const alertId = cb.getAttribute('data-alert-id');
                if (e.target.checked) selectedAlerts.add(alertId);
                else selectedAlerts.delete(alertId);
            });
            updateSelectedCount();
        }

        function clearFilters() {
            document.getElementById('typeFilter').value = 'all';
            document.getElementById('statusFilter').value = 'all';
            document.getElementById('severityFilter').value = 'all';
            document.getElementById('dateFilter').value = '24h';
            document.getElementById('searchAlerts').value = '';

            currentFilters = {
                type: 'all',
                status: 'all',
                severity: 'all',
                dateRange: '24h',
                search: ''
            };

            loadAlerts(1);
        }

        function acknowledgeAlert(alertId, alertType) {
            if (!confirm('Acknowledge this alert?')) return;

            const url = alertType === 'ssl'
                ? `/alerts/ssl/${alertId}/acknowledge`
                : `/alerts/${alertId}/acknowledge`;

            fetch(url, { method: 'POST' })
                .then(response => response.ok ? response.json() : Promise.reject())
                .then(() => {
                    showSuccess('Alert acknowledged');
                    loadAlerts(currentPage);
                })
                .catch(() => showError('Failed to acknowledge alert'));
        }

        function resolveAlert(alertId, alertType) {
            if (!confirm('Resolve this alert?')) return;

            const url = alertType === 'ssl'
                ? `/alerts/ssl/${alertId}/resolve`
                : `/alerts/${alertId}/resolve`;

            fetch(url, { method: 'POST' })
                .then(response => response.ok ? response.json() : Promise.reject())
                .then(() => {
                    showSuccess('Alert resolved');
                    loadAlerts(currentPage);
                })
                .catch(() => showError('Failed to resolve alert'));
        }

        function showAlertDetails(alertId, alertType) {
            const url = alertType === 'ssl'
                ? `/alerts/ssl/${alertId}`
                : `/alerts/${alertId}`;

            fetch(url)
                .then(response => response.json())
                .then(data => {
                    Swal.fire({
                        title: 'Alert Details',
                        html: `
                            <div class="text-left">
                                <p><strong>Type:</strong> ${alertType === 'ssl' ? '🔒 SSL Certificate' : '🌐 Website'}</p>
                                <p><strong>Title:</strong> ${data.title || 'N/A'}</p>
                                <p><strong>Message:</strong> ${data.message || 'N/A'}</p>
                                <p><strong>Severity:</strong> ${data.severity || 'N/A'}</p>
                                <p><strong>Status:</strong> ${data.status || 'N/A'}</p>
                                <p><strong>Monitor:</strong> ${data.monitor_name || 'N/A'}</p>
                                ${data.domain ? `<p><strong>Domain:</strong> ${data.domain}</p>` : ''}
                                ${data.monitor_url ? `<p><strong>URL:</strong> <a href="${data.monitor_url}" target="_blank">${data.monitor_url}</a></p>` : ''}
                                <p><strong>Triggered:</strong> ${new Date(data.triggered_at + 'Z').toLocaleString()}</p>
                                ${data.resolved_at ? `<p><strong>Resolved:</strong> ${new Date(data.resolved_at + 'Z').toLocaleString()}</p>` : ''}
                                ${data.acknowledged_at ? `<p><strong>Acknowledged:</strong> ${new Date(data.acknowledged_at + 'Z').toLocaleString()}</p>` : ''}
                            </div>
                        `,
                        showCloseButton: true,
                        showConfirmButton: false
                    });
                })
                .catch(() => showError('Failed to load alert details'));
        }
        function bulkAcknowledge() {
            if (selectedAlerts.size === 0 || !confirm(`Acknowledge ${selectedAlerts.size} selected alert(s)?`)) return;

            const alertIds = Array.from(selectedAlerts).map(id => parseInt(id));
            fetch('/alerts/bulk-acknowledge', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ alert_ids: alertIds })
            })
            .then(response => response.ok ? response.json() : Promise.reject())
            .then(() => {
                showSuccess(`${selectedAlerts.size} alert(s) acknowledged`);
                selectedAlerts.clear();
                updateSelectedCount();
                loadAlerts(currentPage);
            })
            .catch(() => showError('Failed to acknowledge alerts'));
        }

        function bulkResolve() {
            if (selectedAlerts.size === 0 || !confirm(`Resolve ${selectedAlerts.size} selected alert(s)?`)) return;

            const alertIds = Array.from(selectedAlerts).map(id => parseInt(id));
            fetch('/alerts/bulk-resolve', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ alert_ids: alertIds })
            })
            .then(response => response.ok ? response.json() : Promise.reject())
            .then(() => {
                showSuccess(`${selectedAlerts.size} alert(s) resolved`);
                selectedAlerts.clear();
                updateSelectedCount();
                loadAlerts(currentPage);
            })
            .catch(() => showError('Failed to resolve alerts'));
        }

        function bulkDelete() {
            if (selectedAlerts.size === 0 || !confirm(`Permanently delete ${selectedAlerts.size} selected alert(s)?`)) return;

            const alertIds = Array.from(selectedAlerts).map(id => parseInt(id));
            fetch('/alerts/bulk-delete', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ alert_ids: alertIds })
            })
            .then(response => response.ok ? response.json() : Promise.reject())
            .then(() => {
                showSuccess(`${selectedAlerts.size} alert(s) deleted`);
                selectedAlerts.clear();
                updateSelectedCount();
                loadAlerts(currentPage);
            })
            .catch(() => showError('Failed to delete alerts'));
        }

        function markAllRead() {
            if (!confirm('Mark all alerts as read?')) return;

            fetch('/alerts/mark-all-read', { method: 'POST' })
                .then(response => response.ok ? response.json() : Promise.reject())
                .then(() => {
                    showSuccess('All alerts marked as read');
                    loadAlerts(currentPage);
                })
                .catch(() => showError('Failed to mark alerts as read'));
        }

        function acknowledgeAll() {
            if (!confirm('Acknowledge all active alerts?')) return;

            fetch('/alerts/acknowledge-all', { method: 'POST' })
                .then(response => response.ok ? response.json() : Promise.reject())
                .then(() => {
                    showSuccess('All active alerts acknowledged');
                    loadAlerts(currentPage);
                })
                .catch(() => showError('Failed to acknowledge alerts'));
        }

        function showSuccess(message) {
            Swal.fire({
                icon: 'success',
                title: 'Success!',
                text: message,
                timer: 2000,
                showConfirmButton: false
            });
        }

        function showError(message) {
            Swal.fire({
                icon: 'error',
                title: 'Error',
                text: message
            });
        }

        setInterval(() => {
            if (document.visibilityState === 'visible') loadAlerts(currentPage);
        }, 30000);
    </script>
    </body>
    </html>
    "##);

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

// ==================== STRUCTS FOR QUERIES ====================

#[derive(serde::Deserialize)]
struct AlertQueryParams {
    page: Option<i64>,
    r#type: Option<String>,
    status: Option<String>,
    severity: Option<String>,
    date_range: Option<String>,
    search: Option<String>,
}

#[derive(serde::Deserialize)]
struct BulkActionPayload {
    alert_ids: Vec<i64>,
}

// ==================== DATABASE HANDLERS ====================

async fn alerts_data(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    query: web::Query<AlertQueryParams>,
) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let page = query.page.unwrap_or(1);
                    let per_page = 10;
                    let _offset = (page - 1) * per_page;

                    let alert_type = query.r#type.as_deref().unwrap_or("all");

                    let mut all_alerts = Vec::new();

                    // ========== AMBIL WEBSITE ALERTS ==========
                    if alert_type == "all" || alert_type == "website" {
                        let mut conditions = vec!["a.user_id = ?".to_string()];

                        if let Some(status) = &query.status {
                            if status != "all" {
                                conditions.push(format!("a.status = '{}'", status));
                            }
                        }

                        if let Some(severity) = &query.severity {
                            if severity != "all" {
                                conditions.push(format!("a.severity = '{}'", severity));
                            }
                        }

                        if let Some(date_range) = &query.date_range {
                            if date_range != "all" {
                                let hours = match date_range.as_str() {
                                    "24h" => 24,
                                    "7d" => 168,
                                    "30d" => 720,
                                    _ => 24
                                };
                                conditions.push(format!("a.triggered_at >= datetime('now', '-{} hours')", hours));
                            }
                        }

                        if let Some(search) = &query.search {
                            if !search.is_empty() {
                                conditions.push(format!(
                                    "(a.title LIKE '%{}%' OR a.message LIKE '%{}%' OR m.name LIKE '%{}%')",
                                    search, search, search
                                ));
                            }
                        }

                        let where_clause = conditions.join(" AND ");

                        let website_query = format!(
                            "SELECT
                                a.id,
                                a.title,
                                a.message,
                                a.severity,
                                a.status,
                                a.triggered_at,
                                a.resolved_at,
                                a.acknowledged_at,
                                COALESCE(m.name, 'Unknown') as monitor_name,
                                COALESCE(m.type, 'website') as monitor_type,
                                'website' as alert_type
                             FROM alerts a
                             LEFT JOIN monitors m ON a.monitor_id = m.id
                             WHERE {}
                             ORDER BY a.triggered_at DESC",
                            where_clause
                        );

                        let rows = sqlx::query(&website_query)
                            .bind(user_id)
                            .fetch_all(db_pool.get_ref())
                            .await
                            .unwrap_or(vec![]);

                        for row in rows {
                            all_alerts.push(json!({
                                "id": row.get::<i64, _>("id"),
                                "title": row.get::<String, _>("title"),
                                "message": row.get::<String, _>("message"),
                                "severity": row.get::<String, _>("severity"),
                                "status": row.get::<String, _>("status"),
                                "triggered_at": row.get::<String, _>("triggered_at"),
                                "resolved_at": row.get::<Option<String>, _>("resolved_at"),
                                "acknowledged_at": row.get::<Option<String>, _>("acknowledged_at"),
                                "monitor_name": row.get::<String, _>("monitor_name"),
                                "monitor_type": row.get::<String, _>("monitor_type"),
                                "alert_type": "website"
                            }));
                        }
                    }

                    // ========== AMBIL SSL ALERTS ==========
                    if alert_type == "all" || alert_type == "ssl" {
                        let mut ssl_conditions = vec!["a.user_id = ?".to_string()];

                        if let Some(status) = &query.status {
                            if status != "all" {
                                ssl_conditions.push(format!("a.status = '{}'", status));
                            }
                        }

                        if let Some(severity) = &query.severity {
                            if severity != "all" {
                                ssl_conditions.push(format!("a.severity = '{}'", severity));
                            }
                        }

                        if let Some(date_range) = &query.date_range {
                            if date_range != "all" {
                                let hours = match date_range.as_str() {
                                    "24h" => 24,
                                    "7d" => 168,
                                    "30d" => 720,
                                    _ => 24
                                };
                                ssl_conditions.push(format!("a.triggered_at >= datetime('now', '-{} hours')", hours));
                            }
                        }

                        if let Some(search) = &query.search {
                            if !search.is_empty() {
                                ssl_conditions.push(format!(
                                    "(a.title LIKE '%{}%' OR a.message LIKE '%{}%' OR s.domain LIKE '%{}%')",
                                    search, search, search
                                ));
                            }
                        }

                        let ssl_where_clause = ssl_conditions.join(" AND ");

                        let ssl_query = format!(
                            "SELECT
                                a.id,
                                a.title,
                                a.message,
                                a.severity,
                                a.status,
                                a.triggered_at,
                                a.resolved_at,
                                a.acknowledged_at,
                                s.domain as monitor_name,
                                'ssl' as monitor_type,
                                'ssl' as alert_type
                             FROM ssl_alerts a
                             LEFT JOIN ssl_monitors s ON a.ssl_monitor_id = s.id
                             WHERE {}
                             ORDER BY a.triggered_at DESC",
                            ssl_where_clause
                        );

                        let rows = sqlx::query(&ssl_query)
                            .bind(user_id)
                            .fetch_all(db_pool.get_ref())
                            .await
                            .unwrap_or(vec![]);

                        for row in rows {
                            all_alerts.push(json!({
                                "id": row.get::<i64, _>("id"),
                                "title": row.get::<String, _>("title"),
                                "message": row.get::<String, _>("message"),
                                "severity": row.get::<String, _>("severity"),
                                "status": row.get::<String, _>("status"),
                                "triggered_at": row.get::<String, _>("triggered_at"),
                                "resolved_at": row.get::<Option<String>, _>("resolved_at"),
                                "acknowledged_at": row.get::<Option<String>, _>("acknowledged_at"),
                                "monitor_name": row.get::<String, _>("monitor_name"),
                                "monitor_type": "ssl",
                                "alert_type": "ssl"
                            }));
                        }
                    }

                    // Urutkan berdasarkan triggered_at terbaru
                    all_alerts.sort_by(|a, b| {
                        let a_time = a["triggered_at"].as_str().unwrap_or("");
                        let b_time = b["triggered_at"].as_str().unwrap_or("");
                        b_time.cmp(a_time)
                    });

                    // Pagination
                    let total = all_alerts.len() as i64;
                    let start = ((page - 1) * per_page) as usize;
                    let end = (start + per_page as usize).min(all_alerts.len());
                    let paginated_alerts = all_alerts[start..end].to_vec();
                    let total_pages = if total == 0 { 1 } else { (total as f64 / per_page as f64).ceil() as i64 };

                    let stats = get_alert_stats(db_pool.get_ref(), user_id).await;

                    HttpResponse::Ok().json(json!({
                        "alerts": paginated_alerts,
                        "pagination": {
                            "current_page": page,
                            "per_page": per_page,
                            "total": total,
                            "total_pages": total_pages
                        },
                        "stats": stats
                    }))
                }
                Ok(None) => HttpResponse::Unauthorized().json(json!({
                    "status": "error",
                    "message": "User not found"
                })),
                Err(e) => {
                    eprintln!("Database error: {:?}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "status": "error",
                        "message": "Database error"
                    }))
                }
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn get_alert_stats(db_pool: &SqlitePool, user_id: i64) -> serde_json::Value {

    // Website active alerts
    let website_active = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM alerts WHERE user_id = ? AND status = 'active'"
    )
    .bind(user_id)
    .fetch_one(db_pool)
    .await
    .unwrap_or(0);

    // SSL active alerts
    let ssl_active = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ssl_alerts WHERE user_id = ? AND status = 'active'"
    )
    .bind(user_id)
    .fetch_one(db_pool)
    .await
    .unwrap_or(0);

    let active = website_active + ssl_active;

    // Website warning alerts
    let website_warning = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM alerts
         WHERE user_id = ? AND severity = 'warning' AND status != 'resolved'"
    )
    .bind(user_id)
    .fetch_one(db_pool)
    .await
    .unwrap_or(0);

    // SSL warning alerts
    let ssl_warning = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ssl_alerts
         WHERE user_id = ? AND severity = 'warning' AND status != 'resolved'"
    )
    .bind(user_id)
    .fetch_one(db_pool)
    .await
    .unwrap_or(0);

    let warning = website_warning + ssl_warning;

    // Website resolved in 24h
    let website_resolved = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM alerts
         WHERE user_id = ? AND status = 'resolved'
         AND resolved_at >= datetime('now', '-24 hours')"
    )
    .bind(user_id)
    .fetch_one(db_pool)
    .await
    .unwrap_or(0);

    // SSL resolved in 24h
    let ssl_resolved = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ssl_alerts
         WHERE user_id = ? AND status = 'resolved'
         AND resolved_at >= datetime('now', '-24 hours')"
    )
    .bind(user_id)
    .fetch_one(db_pool)
    .await
    .unwrap_or(0);

    let resolved_24h = website_resolved + ssl_resolved;

    // Avg response time (only for website)
    let avg_result = sqlx::query_scalar::<_, Option<f64>>(
        "SELECT AVG(cr.response_time)
         FROM check_results cr
         JOIN monitors m ON cr.monitor_id = m.id
         WHERE m.user_id = ? AND cr.response_time IS NOT NULL
         AND cr.checked_at >= datetime('now', '-1 hour')"
    )
    .bind(user_id)
    .fetch_one(db_pool)
    .await;

    let avg_response_time = if let Ok(Some(avg)) = avg_result {
        avg as i64
    } else {
        0
    };

    json!({
        "active": active,
        "warning": warning,
        "resolved_24h": resolved_24h,
        "avg_response_time": avg_response_time
    })
}

async fn get_alert(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let alert_id = path.into_inner();

    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let alert = sqlx::query(
                        "SELECT
                            a.id,
                            a.title,
                            a.message,
                            a.severity,
                            a.status,
                            a.triggered_at,
                            a.resolved_at,
                            a.acknowledged_at,
                            m.name as monitor_name,
                            m.type as monitor_type,
                            m.target_url as monitor_url
                         FROM alerts a
                         LEFT JOIN monitors m ON a.monitor_id = m.id
                         WHERE a.id = ? AND a.user_id = ?"
                    )
                    .bind(alert_id)
                    .bind(user_id)
                    .fetch_optional(db_pool.get_ref())
                    .await;

                    match alert {
                        Ok(Some(row)) => {
                            let id: i64 = row.get("id");
                            let title: String = row.get("title");
                            let message: String = row.get("message");
                            let severity: String = row.get("severity");
                            let status: String = row.get("status");
                            let triggered_at: String = row.get("triggered_at");
                            let resolved_at: Option<String> = row.get("resolved_at");
                            let acknowledged_at: Option<String> = row.get("acknowledged_at");
                            let monitor_name: String = row.get("monitor_name");
                            let monitor_type: String = row.get("monitor_type");
                            let monitor_url: Option<String> = row.get("monitor_url");

                            HttpResponse::Ok().json(json!({
                                "id": id,
                                "title": title,
                                "message": message,
                                "severity": severity,
                                "status": status,
                                "triggered_at": triggered_at,
                                "resolved_at": resolved_at,
                                "acknowledged_at": acknowledged_at,
                                "monitor_name": monitor_name,
                                "monitor_type": monitor_type,
                                "monitor_url": monitor_url
                            }))
                        }
                        Ok(None) => HttpResponse::NotFound().json(json!({
                            "status": "error",
                            "message": "Alert not found"
                        })),
                        Err(e) => {
                            eprintln!("Database error: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "status": "error",
                                "message": "Database error"
                            }))
                        }
                    }
                }
                Ok(None) => HttpResponse::Unauthorized().json(json!({
                    "status": "error",
                    "message": "User not found"
                })),
                Err(e) => {
                    eprintln!("Database error: {:?}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "status": "error",
                        "message": "Database error"
                    }))
                }
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn acknowledge_alert(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let alert_id = path.into_inner();

    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let result = sqlx::query(
                        "UPDATE alerts
                         SET status = 'acknowledged', acknowledged_at = datetime('now')
                         WHERE id = ? AND user_id = ? AND status = 'active'"
                    )
                    .bind(alert_id)
                    .bind(user_id)
                    .execute(db_pool.get_ref())
                    .await;

                    match result {
                        Ok(rows) => {
                            if rows.rows_affected() > 0 {
                                HttpResponse::Ok().json(json!({
                                    "status": "success",
                                    "message": format!("Alert {} acknowledged", alert_id)
                                }))
                            } else {
                                HttpResponse::NotFound().json(json!({
                                    "status": "error",
                                    "message": "Alert not found or already acknowledged"
                                }))
                            }
                        }
                        Err(e) => {
                            eprintln!("Database error: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "status": "error",
                                "message": "Database error"
                            }))
                        }
                    }
                }
                _ => HttpResponse::Unauthorized().json(json!({
                    "status": "error",
                    "message": "Authentication failed"
                }))
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn resolve_alert(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let alert_id = path.into_inner();

    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let result = sqlx::query(
                        "UPDATE alerts
                         SET status = 'resolved', resolved_at = datetime('now')
                         WHERE id = ? AND user_id = ? AND status != 'resolved'"
                    )
                    .bind(alert_id)
                    .bind(user_id)
                    .execute(db_pool.get_ref())
                    .await;

                    match result {
                        Ok(rows) => {
                            if rows.rows_affected() > 0 {
                                HttpResponse::Ok().json(json!({
                                    "status": "success",
                                    "message": format!("Alert {} resolved", alert_id)
                                }))
                            } else {
                                HttpResponse::NotFound().json(json!({
                                    "status": "error",
                                    "message": "Alert not found or already resolved"
                                }))
                            }
                        }
                        Err(e) => {
                            eprintln!("Database error: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "status": "error",
                                "message": "Database error"
                            }))
                        }
                    }
                }
                _ => HttpResponse::Unauthorized().json(json!({
                    "status": "error",
                    "message": "Authentication failed"
                }))
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn bulk_acknowledge(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    payload: web::Json<BulkActionPayload>,
) -> impl Responder {
    let alert_ids = &payload.alert_ids;

    if alert_ids.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "status": "error",
            "message": "No alert IDs provided"
        }));
    }

    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let placeholders: Vec<String> = (0..alert_ids.len()).map(|_| "?".to_string()).collect();
                    let query = format!(
                        "UPDATE alerts
                         SET status = 'acknowledged', acknowledged_at = datetime('now')
                         WHERE user_id = ? AND id IN ({}) AND status = 'active'",
                        placeholders.join(",")
                    );

                    let mut query_builder = sqlx::query(&query);
                    query_builder = query_builder.bind(user_id);

                    for id in alert_ids {
                        query_builder = query_builder.bind(id);
                    }

                    let result = query_builder.execute(db_pool.get_ref()).await;

                    match result {
                        Ok(rows) => {
                            let affected = rows.rows_affected();
                            HttpResponse::Ok().json(json!({
                                "status": "success",
                                "message": format!("{} alert(s) acknowledged", affected)
                            }))
                        }
                        Err(e) => {
                            eprintln!("Database error: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "status": "error",
                                "message": "Database error"
                            }))
                        }
                    }
                }
                _ => HttpResponse::Unauthorized().json(json!({
                    "status": "error",
                    "message": "Authentication failed"
                }))
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn bulk_resolve(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    payload: web::Json<BulkActionPayload>,
) -> impl Responder {
    let alert_ids = &payload.alert_ids;

    if alert_ids.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "status": "error",
            "message": "No alert IDs provided"
        }));
    }

    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let placeholders: Vec<String> = (0..alert_ids.len()).map(|_| "?".to_string()).collect();
                    let query = format!(
                        "UPDATE alerts
                         SET status = 'resolved', resolved_at = datetime('now')
                         WHERE user_id = ? AND id IN ({}) AND status != 'resolved'",
                        placeholders.join(",")
                    );

                    let mut query_builder = sqlx::query(&query);
                    query_builder = query_builder.bind(user_id);

                    for id in alert_ids {
                        query_builder = query_builder.bind(id);
                    }

                    let result = query_builder.execute(db_pool.get_ref()).await;

                    match result {
                        Ok(rows) => {
                            let affected = rows.rows_affected();
                            HttpResponse::Ok().json(json!({
                                "status": "success",
                                "message": format!("{} alert(s) resolved", affected)
                            }))
                        }
                        Err(e) => {
                            eprintln!("Database error: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "status": "error",
                                "message": "Database error"
                            }))
                        }
                    }
                }
                _ => HttpResponse::Unauthorized().json(json!({
                    "status": "error",
                    "message": "Authentication failed"
                }))
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn bulk_delete(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    payload: web::Json<BulkActionPayload>,
) -> impl Responder {
    let alert_ids = &payload.alert_ids;

    if alert_ids.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "status": "error",
            "message": "No alert IDs provided"
        }));
    }

    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let placeholders: Vec<String> = (0..alert_ids.len()).map(|_| "?".to_string()).collect();
                    let query = format!(
                        "DELETE FROM alerts WHERE user_id = ? AND id IN ({})",
                        placeholders.join(",")
                    );

                    let mut query_builder = sqlx::query(&query);
                    query_builder = query_builder.bind(user_id);

                    for id in alert_ids {
                        query_builder = query_builder.bind(id);
                    }

                    let result = query_builder.execute(db_pool.get_ref()).await;

                    match result {
                        Ok(rows) => {
                            let affected = rows.rows_affected();
                            HttpResponse::Ok().json(json!({
                                "status": "success",
                                "message": format!("{} alert(s) deleted", affected)
                            }))
                        }
                        Err(e) => {
                            eprintln!("Database error: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "status": "error",
                                "message": "Database error"
                            }))
                        }
                    }
                }
                _ => HttpResponse::Unauthorized().json(json!({
                    "status": "error",
                    "message": "Authentication failed"
                }))
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn mark_all_read(
    session: Session,
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let result = sqlx::query(
                        "UPDATE alerts
                         SET status = 'acknowledged', acknowledged_at = datetime('now')
                         WHERE user_id = ? AND status = 'active'"
                    )
                    .bind(user_id)
                    .execute(db_pool.get_ref())
                    .await;

                    match result {
                        Ok(rows) => {
                            let affected = rows.rows_affected();
                            HttpResponse::Ok().json(json!({
                                "status": "success",
                                "message": format!("{} alert(s) marked as read", affected)
                            }))
                        }
                        Err(e) => {
                            eprintln!("Database error: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "status": "error",
                                "message": "Database error"
                            }))
                        }
                    }
                }
                _ => HttpResponse::Unauthorized().json(json!({
                    "status": "error",
                    "message": "Authentication failed"
                }))
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn acknowledge_all(
    session: Session,
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    mark_all_read(session, db_pool).await
}
