use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::{SqlitePool, Row};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// INCLUDE SEPERTI PHP
use crate::navbar;
use crate::sidebar;
use crate::footer;

// ==================== STRUCT UNTUK QUERY (MATCH DATABASE) ====================
#[derive(Debug, sqlx::FromRow)]
pub struct Monitor {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub target_url: String,
    #[sqlx(rename = "type")]
    pub monitor_type: String,
    pub check_interval: i64,
    pub timeout: i64,
    pub is_maintenance: bool,
    pub is_active: bool,
    pub created_at: String,
}

// ==================== STRUCT UNTUK RENDER HTML ====================
#[derive(Serialize)]
pub struct MonitorRow {
    pub id: i64,
    pub name: String,
    pub target_url: String,
    pub monitor_type: String,
    pub check_interval: i64,
    pub is_active: bool,
    pub last_status: Option<String>,
    pub last_response_time: Option<i64>,
    pub last_checked: Option<DateTime<Utc>>,
    pub timeout: i64,
    pub is_maintenance: bool,
    pub created_at: Option<DateTime<Utc>>,
}

// ==================== STRUCT UNTUK REALTIME ====================
#[derive(Serialize)]
struct MonitorStatus {
    id: i64,
    status: Option<String>,
    response_time: Option<i64>,
    last_checked: Option<DateTime<Utc>>,
}

// ==================== QUERY PARAMS ====================
#[derive(Deserialize)]
pub struct MonitorQuery {
    pub page: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub type_filter: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct MonitorJson {
    pub monitors: Vec<MonitorData>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MonitorData {
    pub name: String,
    pub url: String,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub interval: Option<String>,
}

#[derive(Deserialize)]
struct StatusQuery {
    ids: String,
}

// ==================== ROUTES ====================
pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/monitors")
            .route("", web::get().to(monitors_page))
            .route("", web::post().to(create_monitor_json))
            .route("/data", web::get().to(monitors_data))
            .route("/{id}/toggle", web::post().to(toggle_monitor))
            .route("/{id}/delete", web::post().to(delete_monitor))
            .route("/{id}/edit", web::get().to(get_monitor_for_edit))
            .route("/{id}/update", web::post().to(update_monitor))
            .route("/recent", web::get().to(recent_monitors))
            .route("/status-updates", web::get().to(monitor_status_updates))
    );
}

// ==================== PAGE RENDER ====================
async fn monitors_page(session: Session) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => render_monitors_page(&email),
        _ => HttpResponse::Found()
            .append_header(("Location", "/"))
            .finish(),
    }
}

fn render_monitors_page(email: &str) -> HttpResponse {
    let mut html = String::new();

    html.push_str(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Monitors - Basic Monitoring</title>
    <link href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/sweetalert2@11"></script>
    <script src="https://cdn.jsdelivr.net/npm/toastify-js"></script>
    <link rel="stylesheet" type="text/css" href="https://cdn.jsdelivr.net/npm/toastify-js/src/toastify.min.css">
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
        body { font-family: 'Inter', sans-serif; background-color: #f8fafc; }
        .sidebar-bg { background: linear-gradient(180deg, #1e293b 0%, #0f172a 100%); }
        .status-up { color: #10b981; }
        .status-down { color: #ef4444; }
        .status-slow { color: #f59e0b; }
        .status-unknown { color: #6b7280; }
        .pagination-active { background-color: #3b82f6; color: white; }
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
        .modal-overlay.active {
            display: flex;
        }
        .realtime-indicator {
            display: inline-flex;
            align-items: center;
            font-size: 0.75rem;
            padding: 2px 8px;
            border-radius: 9999px;
            background-color: #dcfce7;
            color: #166534;
            margin-left: 8px;
        }
        .realtime-indicator.blinking {
            animation: blink 1.5s infinite;
        }
        @keyframes blink {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
        }
        .status-dot {
            display: inline-block;
            width: 10px;
            height: 10px;
            border-radius: 50%;
            margin-right: 6px;
        }
        .status-dot.up { background-color: #10b981; }
        .status-dot.down { background-color: #ef4444; }
        .status-dot.slow { background-color: #f59e0b; }
        .status-dot.unknown { background-color: #6b7280; }
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
                <h1 class="text-3xl font-bold text-slate-800">Website Monitors</h1>
                <p class="text-slate-600 mt-2">Monitor and manage your websites in real-time</p>
            </div>
            <div class="mt-4 md:mt-0 flex items-center space-x-4">
                <div id="realtime-status" class="realtime-indicator hidden">
                    <div class="animate-spin rounded-full h-3 w-3 border-t-2 border-b-2 border-green-500 mr-1"></div>
                    <span>Live Updates</span>
                </div>
                <button onclick="openAddMonitorModal()"
                        class="inline-flex items-center bg-blue-600 text-white font-semibold px-5 py-2.5 rounded-lg hover:bg-blue-700">
                    <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
                    </svg>
                    Add Monitor
                </button>
            </div>
        </div>

        <div class="bg-white rounded-xl shadow p-4 mb-6">
            <form id="filter-form" class="grid grid-cols-1 md:grid-cols-4 gap-4">
                <div>
                    <label class="block text-slate-700 text-sm font-medium mb-2">Search</label>
                    <input type="text" id="search-input" placeholder="Search by name or URL..."
                           class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                </div>
                <div>
                    <label class="block text-slate-700 text-sm font-medium mb-2">Status</label>
                    <select id="status-select" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                        <option value="">All Status</option>
                        <option value="up">Up</option>
                        <option value="down">Down</option>
                        <option value="slow">Slow</option>
                        <option value="unknown">Unknown</option>
                    </select>
                </div>
                <div>
                    <label class="block text-slate-700 text-sm font-medium mb-2">Type</label>
                    <select id="type-select" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                        <option value="">All Types</option>
                        <option value="http">HTTP</option>
                        <option value="https">HTTPS</option>
                        <option value="ping">PING</option>
                    </select>
                </div>
                <div class="flex items-end space-x-2">
                    <button type="button" onclick="applyFilters()"
                            class="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700">
                        Apply Filters
                    </button>
                    <button type="button" onclick="resetFilters()"
                            class="w-full px-4 py-2 border border-slate-300 text-slate-700 rounded-lg hover:bg-slate-50">
                        Reset
                    </button>
                </div>
            </form>
        </div>

        <div class="bg-white rounded-xl shadow overflow-hidden">
            <div class="p-6 border-b border-slate-200 flex justify-between items-center">
                <h2 class="text-xl font-bold text-slate-800">Your Monitors</h2>
                <div class="flex items-center space-x-4">
                    <div class="text-sm text-slate-500" id="last-update-time">
                        Last update: <span id="update-time">Just now</span>
                    </div>
                    <div class="loading-indicator flex items-center">
                        <div class="animate-spin rounded-full h-5 w-5 border-t-2 border-b-2 border-blue-500 mr-2"></div>
                        <span class="text-sm text-slate-500">Loading...</span>
                    </div>
                </div>
            </div>

            <div id="monitors-table">
                <div class="p-12 text-center">
                    <div class="inline-block animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500"></div>
                    <p class="mt-2 text-slate-500">Loading monitors...</p>
                </div>
            </div>
        </div>
    </div>
</div>"##);

    // MODAL ADD MONITOR
    html.push_str(r##"
    <div id="add-monitor-modal" class="modal-overlay">
        <div class="bg-white rounded-2xl w-full max-w-2xl max-h-[90vh] overflow-auto">
            <div class="p-6 border-b border-slate-200 sticky top-0 bg-white">
                <div class="flex justify-between items-center">
                    <h3 class="text-xl font-bold text-slate-800">Add New Monitor</h3>
                    <button onclick="closeAddMonitorModal()" class="text-slate-400 hover:text-slate-600">
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>
                <p class="text-slate-600 mt-1">Add one or multiple websites to monitor</p>
            </div>

            <form id="add-monitor-form" class="p-6">
                <div id="monitor-fields">
                    <div class="monitor-field bg-slate-50 p-4 rounded-lg mb-4">
                        <div class="flex justify-between items-center mb-3">
                            <span class="text-slate-700 font-medium">Monitor #1</span>
                            <button type="button" onclick="removeMonitorField(this)" class="text-red-500 hover:text-red-700 hidden">
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                                </svg>
                            </button>
                        </div>
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">Display Name *</label>
                                <input type="text" name="name"
                                       class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                                       placeholder="My Website" required>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">URL *</label>
                                <input type="url" name="url"
                                       class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                                       placeholder="https://example.com" required>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">Type</label>
                                <select name="type" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                                    <option value="http">HTTP</option>
                                    <option value="https">HTTPS</option>
                                    <option value="ping">PING</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">Check Interval</label>
                                <select name="interval" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                                    <option value="30">30 seconds</option>
                                    <option value="60" selected>60 seconds</option>
                                    <option value="120">2 minutes</option>
                                    <option value="300">5 minutes</option>
                                    <option value="600">10 minutes</option>
                                </select>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="flex justify-between items-center mt-6">
                    <button type="button" onclick="addMoreMonitorField()"
                            class="flex items-center text-blue-600 hover:text-blue-800">
                        <svg class="w-5 h-5 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
                        </svg>
                        Add Another URL
                    </button>

                    <div class="flex space-x-3">
                        <button type="button" onclick="closeAddMonitorModal()"
                                class="px-6 py-2 border border-slate-300 text-slate-700 rounded-lg hover:bg-slate-50">
                            Cancel
                        </button>
                        <button type="button" onclick="submitMonitorForm()"
                                class="px-6 py-2 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700">
                            Save Monitors
                        </button>
                    </div>
                </div>
            </form>
        </div>
    </div>

    <!-- MODAL EDIT MONITOR -->
    <div id="edit-monitor-modal" class="modal-overlay">
        <div class="bg-white rounded-2xl w-full max-w-2xl max-h-[90vh] overflow-auto">
            <div class="p-6 border-b border-slate-200 sticky top-0 bg-white">
                <div class="flex justify-between items-center">
                    <h3 class="text-xl font-bold text-slate-800">✏️ Edit Monitor</h3>
                    <button onclick="closeEditMonitorModal()" class="text-slate-400 hover:text-slate-600">
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>
                <p class="text-slate-600 mt-1">Update monitor configuration</p>
            </div>

            <form id="edit-monitor-form" class="p-6">
                <div class="monitor-field bg-slate-50 p-4 rounded-lg">
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div>
                            <label class="block text-slate-700 font-medium mb-2">Display Name *</label>
                            <input type="text" id="edit-name" name="name"
                                   class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                                   placeholder="My Website" required>
                        </div>
                        <div>
                            <label class="block text-slate-700 font-medium mb-2">URL *</label>
                            <input type="url" id="edit-url" name="url"
                                   class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                                   placeholder="https://example.com" required>
                        </div>
                        <div>
                            <label class="block text-slate-700 font-medium mb-2">Type</label>
                            <select id="edit-type" name="type" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                                <option value="http">HTTP</option>
                                <option value="https">HTTPS</option>
                                <option value="ping">PING</option>
                            </select>
                        </div>
                        <div>
                            <label class="block text-slate-700 font-medium mb-2">Check Interval</label>
                            <select id="edit-interval" name="interval" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                                <option value="30">30 seconds</option>
                                <option value="60">60 seconds</option>
                                <option value="120">2 minutes</option>
                                <option value="300">5 minutes</option>
                                <option value="600">10 minutes</option>
                            </select>
                        </div>
                    </div>
                </div>

                <div class="flex justify-end space-x-3 mt-6 pt-4 border-t">
                    <button type="button" onclick="closeEditMonitorModal()"
                            class="px-6 py-2 border border-slate-300 text-slate-700 rounded-lg hover:bg-slate-50">
                        Cancel
                    </button>
                    <button type="button" onclick="submitEditMonitorForm()"
                            class="px-6 py-2 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700">
                        Update Monitor
                    </button>
                </div>
            </form>
        </div>
    </div>

    <script>
        let monitorCounter = 1;
        let currentPage = 1;
        let pollingInterval = null;
        let isPollingActive = true;
        let lastUpdateTime = new Date();
        let monitorIds = new Set();

        function openAddMonitorModal() {
            const modal = document.getElementById('add-monitor-modal');
            if (modal) {
                modal.classList.add('active');
                document.getElementById('monitor-fields').innerHTML = `
                    <div class="monitor-field bg-slate-50 p-4 rounded-lg mb-4">
                        <div class="flex justify-between items-center mb-3">
                            <span class="text-slate-700 font-medium">Monitor #1</span>
                            <button type="button" onclick="removeMonitorField(this)" class="text-red-500 hover:text-red-700 hidden">
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                                </svg>
                            </button>
                        </div>
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">Display Name *</label>
                                <input type="text" name="name"
                                       class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                                       placeholder="My Website" required>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">URL *</label>
                                <input type="url" name="url"
                                       class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                                       placeholder="https://example.com" required>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">Type</label>
                                <select name="type" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                                    <option value="http">HTTP</option>
                                    <option value="https">HTTPS</option>
                                    <option value="ping">PING</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">Check Interval</label>
                                <select name="interval" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                                    <option value="30">30 seconds</option>
                                    <option value="60" selected>60 seconds</option>
                                    <option value="120">2 minutes</option>
                                    <option value="300">5 minutes</option>
                                    <option value="600">10 minutes</option>
                                </select>
                            </div>
                        </div>
                    </div>
                `;
                monitorCounter = 1;
            }
        }

        function closeAddMonitorModal() {
            document.getElementById('add-monitor-modal')?.classList.remove('active');
        }

        function closeEditMonitorModal() {
            document.getElementById('edit-monitor-modal')?.classList.remove('active');
        }

        function addMoreMonitorField() {
            const fieldsContainer = document.getElementById('monitor-fields');
            monitorCounter++;
            const newField = document.createElement('div');
            newField.className = 'monitor-field bg-slate-50 p-4 rounded-lg mb-4';
            newField.innerHTML = `
                <div class="flex justify-between items-center mb-3">
                    <span class="text-slate-700 font-medium">Monitor #${monitorCounter}</span>
                    <button type="button" onclick="removeMonitorField(this)" class="text-red-500 hover:text-red-700">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                        </svg>
                    </button>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label class="block text-slate-700 font-medium mb-2">Display Name *</label>
                        <input type="text" name="name"
                               class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                               placeholder="My Website" required>
                    </div>
                    <div>
                        <label class="block text-slate-700 font-medium mb-2">URL *</label>
                        <input type="url" name="url"
                               class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                               placeholder="https://example.com" required>
                    </div>
                    <div>
                        <label class="block text-slate-700 font-medium mb-2">Type</label>
                        <select name="type" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                            <option value="http">HTTP</option>
                            <option value="https">HTTPS</option>
                            <option value="ping">PING</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-slate-700 font-medium mb-2">Check Interval</label>
                        <select name="interval" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                            <option value="30">30 seconds</option>
                            <option value="60" selected>60 seconds</option>
                            <option value="120">2 minutes</option>
                            <option value="300">5 minutes</option>
                            <option value="600">10 minutes</option>
                        </select>
                    </div>
                </div>
            `;
            fieldsContainer.appendChild(newField);
        }

        function removeMonitorField(button) {
            if (document.querySelectorAll('.monitor-field').length > 1) {
                button.closest('.monitor-field').remove();
                const fields = document.querySelectorAll('.monitor-field');
                fields.forEach((field, index) => {
                    field.querySelector('.text-slate-700.font-medium').textContent = 'Monitor #' + (index + 1);
                });
                monitorCounter = fields.length;
            }
        }

        function submitMonitorForm() {
            const form = document.getElementById('add-monitor-form');
            const monitors = [];
            const monitorFields = form.querySelectorAll('.monitor-field');

            monitorFields.forEach(field => {
                const nameInput = field.querySelector('input[name="name"]');
                const urlInput = field.querySelector('input[name="url"]');
                const typeSelect = field.querySelector('select[name="type"]');
                const intervalSelect = field.querySelector('select[name="interval"]');

                if (nameInput?.value.trim() && urlInput?.value.trim()) {
                    monitors.push({
                        name: nameInput.value.trim(),
                        url: urlInput.value.trim(),
                        type: typeSelect?.value || 'http',
                        interval: intervalSelect?.value || '60'
                    });
                }
            });

            if (monitors.length === 0) {
                Swal.fire({ icon: 'error', title: 'Error', text: 'Please enter at least one monitor' });
                return;
            }

            fetch('/monitors', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ monitors })
            })
            .then(response => response.text())
            .then(() => {
                closeAddMonitorModal();
                Swal.fire({ icon: 'success', title: 'Success!', text: `${monitors.length} monitor(s) added`, timer: 2000, showConfirmButton: false });
                loadMonitorsData();
            })
            .catch(error => {
                Swal.fire({ icon: 'error', title: 'Error', text: error.message });
            });
        }

        function submitEditMonitorForm() {
            const monitorId = document.getElementById('edit-monitor-modal').getAttribute('data-monitor-id');

            const name = document.getElementById('edit-name').value.trim();
            const url = document.getElementById('edit-url').value.trim();
            const type = document.getElementById('edit-type').value;
            const interval = document.getElementById('edit-interval').value;

            if (!name || !url) {
                Swal.fire({ icon: 'error', title: 'Error', text: 'Name and URL are required' });
                return;
            }

            Swal.fire({
                title: 'Update Monitor',
                text: 'Are you sure you want to update this monitor?',
                icon: 'question',
                showCancelButton: true,
                confirmButtonText: 'Yes, Update',
                cancelButtonText: 'Cancel'
            }).then((result) => {
                if (result.isConfirmed) {
                    fetch(`/monitors/${monitorId}/update`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ name, url, type, interval })
                    })
                    .then(response => response.json())
                    .then(data => {
                        if (data.success) {
                            closeEditMonitorModal();
                            Swal.fire({ icon: 'success', title: 'Updated!', text: 'Monitor updated successfully', timer: 2000, showConfirmButton: false });
                            loadMonitorsData();
                        } else {
                            Swal.fire({ icon: 'error', title: 'Error', text: data.message });
                        }
                    })
                    .catch(error => {
                        Swal.fire({ icon: 'error', title: 'Error', text: error.message });
                    });
                }
            });
        }

        function editMonitor(monitorId) {
            fetch(`/monitors/${monitorId}/edit`)
                .then(response => response.json())
                .then(data => {
                    if (data.success) {
                        const modal = document.getElementById('edit-monitor-modal');
                        modal.setAttribute('data-monitor-id', monitorId);
                        document.getElementById('edit-name').value = data.name;
                        document.getElementById('edit-url').value = data.target_url;
                        document.getElementById('edit-type').value = data.type;
                        document.getElementById('edit-interval').value = data.check_interval;
                        modal.classList.add('active');
                    } else {
                        Swal.fire({ icon: 'error', title: 'Error', text: data.message });
                    }
                })
                .catch(error => {
                    console.error('Error:', error);
                    Swal.fire({ icon: 'error', title: 'Error', text: 'Failed to load monitor data' });
                });
        }

        function applyFilters() {
            currentPage = 1;
            loadMonitorsData();
        }

        function resetFilters() {
            document.getElementById('search-input').value = '';
            document.getElementById('status-select').value = '';
            document.getElementById('type-select').value = '';
            currentPage = 1;
            loadMonitorsData();
        }

        function loadMonitorsData() {
            const search = document.getElementById('search-input').value;
            const status = document.getElementById('status-select').value;
            const typeFilter = document.getElementById('type-select').value;

            let query = `?page=${currentPage}`;
            if (search) query += `&search=${encodeURIComponent(search)}`;
            if (status) query += `&status=${encodeURIComponent(status)}`;
            if (typeFilter) query += `&type_filter=${encodeURIComponent(typeFilter)}`;

            fetch(`/monitors/data${query}`)
                .then(response => response.text())
                .then(html => {
                    document.getElementById('monitors-table').innerHTML = html;
                    extractMonitorIds();
                    updateLastUpdateTime();

                    if (!pollingInterval) {
                        startPolling();
                    }
                })
                .catch(error => console.error('Error loading monitors:', error));
        }

        function extractMonitorIds() {
            monitorIds.clear();
            document.querySelectorAll('tr[id^="monitor-row-"]').forEach(row => {
                const id = row.id.replace('monitor-row-', '');
                monitorIds.add(parseInt(id));
            });
        }

        function updateLastUpdateTime() {
            const now = new Date();
            document.getElementById('update-time').textContent = formatTime(now);
        }

        function formatTime(date) {
            const diff = Math.floor((new Date() - date) / 1000);
            if (diff < 10) return 'Just now';
            if (diff < 60) return `${diff} seconds ago`;
            if (diff < 3600) return `${Math.floor(diff / 60)} minutes ago`;
            return date.toLocaleTimeString();
        }

        function startPolling() {
            if (pollingInterval) clearInterval(pollingInterval);
            pollingInterval = setInterval(() => {
                if (monitorIds.size > 0) {
                    fetchStatusUpdates();
                }
            }, 5000);
        }

        function fetchStatusUpdates() {
            if (monitorIds.size === 0) return;

            const idsString = Array.from(monitorIds).join(',');

            fetch(`/monitors/status-updates?ids=${idsString}`)
                .then(response => response.json())
                .then(data => {
                    updateMonitorStatuses(data);
                    updateLastUpdateTime();
                })
                .catch(error => console.error('Error fetching status:', error));
        }

        function updateMonitorStatuses(statuses) {
            statuses.forEach(status => {
                const row = document.getElementById(`monitor-row-${status.id}`);
                if (row) {
                    const statusCell = row.querySelector('td:nth-child(5) span');
                    if (statusCell) {
                        const dotClass = getStatusDotClass(status.status);
                        statusCell.className = `${getStatusClass(status.status)} font-medium flex items-center`;
                        statusCell.innerHTML = `<span class="status-dot ${dotClass}"></span>${getStatusText(status.status)}`;
                    }

                    const responseCell = row.querySelector(`#response-${status.id}`);
                    if (responseCell) {
                        responseCell.textContent = status.response_time ? `${status.response_time}ms` : 'N/A';
                    }

                    const lastCheckedCell = row.querySelector(`#last-checked-${status.id}`);
                    if (lastCheckedCell && status.last_checked) {
                        lastCheckedCell.textContent = formatTime(new Date(status.last_checked));
                    }
                }
            });
        }

        function getStatusClass(status) {
            switch(status) {
                case 'up': return 'status-up';
                case 'down': return 'status-down';
                case 'slow': return 'status-slow';
                default: return 'status-unknown';
            }
        }

        function getStatusDotClass(status) {
            switch(status) {
                case 'up': return 'up';
                case 'down': return 'down';
                case 'slow': return 'slow';
                default: return 'unknown';
            }
        }

        function getStatusText(status) {
            switch(status) {
                case 'up': return 'Up';
                case 'down': return 'Down';
                case 'slow': return 'Slow';
                default: return 'Unknown';
            }
        }

        function loadPage(pageNum) {
            currentPage = pageNum;
            loadMonitorsData();
        }

        function toggleMonitorStatus(monitorId, currentStatus) {
            Swal.fire({
                title: 'Change Monitor Status',
                text: `Are you sure you want to ${currentStatus ? 'disable' : 'enable'} this monitor?`,
                icon: 'warning',
                showCancelButton: true,
                confirmButtonColor: '#3085d6',
                cancelButtonColor: '#d33',
                confirmButtonText: 'Yes!'
            }).then((result) => {
                if (result.isConfirmed) {
                    fetch(`/monitors/${monitorId}/toggle`, { method: 'POST' })
                        .then(response => {
                            if (response.ok) {
                                loadMonitorsData();
                                Swal.fire('Changed!', `Monitor ${currentStatus ? 'disabled' : 'enabled'}`, 'success');
                            }
                        });
                }
            });
        }

        function deleteMonitor(monitorId) {
            Swal.fire({
                title: 'Delete Monitor',
                text: 'This action cannot be undone!',
                icon: 'error',
                showCancelButton: true,
                confirmButtonColor: '#d33',
                confirmButtonText: 'Delete'
            }).then((result) => {
                if (result.isConfirmed) {
                    fetch(`/monitors/${monitorId}/delete`, { method: 'POST' })
                        .then(response => {
                            if (response.ok) {
                                monitorIds.delete(monitorId);
                                loadMonitorsData();
                                Swal.fire('Deleted!', 'Monitor has been deleted.', 'success');
                            }
                        });
                }
            });
        }

        function copyPublicLink(monitorId) {
            const url = window.location.origin + '/status/' + monitorId;
            navigator.clipboard.writeText(url);
            Swal.fire({
                icon: 'success',
                title: 'Link Copied!',
                text: 'Public status link copied to clipboard',
                timer: 1500,
                showConfirmButton: false
            });
        }

        document.addEventListener('DOMContentLoaded', function() {
            loadMonitorsData();
            setTimeout(() => startPolling(), 1000);

            document.addEventListener('keydown', function(e) {
                if (e.key === 'Escape') {
                    closeAddMonitorModal();
                    closeEditMonitorModal();
                }
            });

            document.addEventListener('click', function(e) {
                const editModal = document.getElementById('edit-monitor-modal');
                const addModal = document.getElementById('add-monitor-modal');
                if (editModal && editModal.classList.contains('active')) {
                    if (e.target === editModal) closeEditMonitorModal();
                }
                if (addModal && addModal.classList.contains('active')) {
                    if (e.target === addModal) closeAddMonitorModal();
                }
            });
        });

        document.addEventListener('visibilitychange', function() {
            if (document.hidden) {
                isPollingActive = false;
            } else {
                isPollingActive = true;
                fetchStatusUpdates();
            }
        });

        window.openAddMonitorModal = openAddMonitorModal;
        window.closeAddMonitorModal = closeAddMonitorModal;
        window.closeEditMonitorModal = closeEditMonitorModal;
        window.addMoreMonitorField = addMoreMonitorField;
        window.removeMonitorField = removeMonitorField;
        window.submitMonitorForm = submitMonitorForm;
        window.submitEditMonitorForm = submitEditMonitorForm;
        window.editMonitor = editMonitor;
        window.applyFilters = applyFilters;
        window.resetFilters = resetFilters;
        window.loadPage = loadPage;
        window.toggleMonitorStatus = toggleMonitorStatus;
        window.deleteMonitor = deleteMonitor;
        window.loadMonitorsData = loadMonitorsData;
        window.copyPublicLink = copyPublicLink;
    </script>
"##);

    html.push_str(&footer::render());
    html.push_str("</body></html>");

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

// ==================== CREATE MONITOR ====================
async fn create_monitor_json(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    monitor_data: web::Json<MonitorJson>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let mut inserted_count = 0;
            let mut errors = Vec::new();

            for monitor in &monitor_data.monitors {
                let monitor_type = monitor.type_.clone().unwrap_or_else(|| "http".to_string());
                let interval = monitor.interval.clone().unwrap_or_else(|| "60".to_string());
                let interval_num = match interval.parse::<i64>() {
                    Ok(num) => num,
                    Err(_) => {
                        errors.push(format!("Invalid interval for monitor '{}': {}", monitor.name, interval));
                        continue;
                    }
                };

                if monitor.url.is_empty() {
                    errors.push(format!("URL is required for monitor '{}'", monitor.name));
                    continue;
                }

                match sqlx::query!(
                    "INSERT INTO monitors (user_id, name, target_url, type, check_interval, timeout, is_maintenance, is_active)
                     VALUES (?, ?, ?, ?, ?, 10, 0, 1)",
                    user_id,
                    monitor.name,
                    monitor.url,
                    monitor_type,
                    interval_num
                )
                .execute(db_pool.get_ref())
                .await
                {
                    Ok(result) => {
                        println!("✅ Inserted monitor ID: {}", result.last_insert_rowid());
                        inserted_count += 1;
                    }
                    Err(e) => {
                        errors.push(format!("Failed to insert monitor '{}': {}", monitor.name, e));
                    }
                }
            }

            if !errors.is_empty() {
                HttpResponse::BadRequest().body(format!("Created {} monitor(s). Errors: {}", inserted_count, errors.join("; ")))
            } else {
                HttpResponse::Ok().body(format!("Created {} monitor(s) successfully", inserted_count))
            }
        }
        _ => HttpResponse::Unauthorized().body("Unauthorized"),
    }
}

// ==================== GET MONITOR FOR EDIT ====================
async fn get_monitor_for_edit(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    monitor_id: web::Path<i64>,
) -> impl Responder {
    let id = monitor_id.into_inner();

    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let monitor = sqlx::query!(
                "SELECT id, name, target_url, type, check_interval, timeout, is_active, is_maintenance
                 FROM monitors
                 WHERE id = ? AND user_id = ?",
                id,
                user_id
            )
            .fetch_optional(db_pool.get_ref())
            .await;

            match monitor {
                Ok(Some(m)) => {
                    HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "id": m.id,
                        "name": m.name,
                        "target_url": m.target_url,
                        "type": m.r#type,
                        "check_interval": m.check_interval,
                        "timeout": m.timeout,
                        "is_active": m.is_active.unwrap_or(true),
                        "is_maintenance": m.is_maintenance.unwrap_or(false)
                    }))
                }
                Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "message": "Monitor not found"
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "success": false,
                    "message": format!("Database error: {}", e)
                }))
            }
        }
        _ => HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Unauthorized"
        })),
    }
}

// ==================== UPDATE MONITOR ====================
async fn update_monitor(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    monitor_id: web::Path<i64>,
    form: web::Json<MonitorData>,
) -> impl Responder {
    let id = monitor_id.into_inner();

    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let exists = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM monitors WHERE id = ? AND user_id = ?",
                id,
                user_id
            )
            .fetch_one(db_pool.get_ref())
            .await
            .unwrap_or(0);

            if exists == 0 {
                return HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "message": "Monitor not found"
                }));
            }

            let monitor_type = form.type_.clone().unwrap_or_else(|| "http".to_string());
            let interval = form.interval.clone().unwrap_or_else(|| "60".to_string());
            let interval_num = match interval.parse::<i64>() {
                Ok(num) => num,
                Err(_) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "success": false,
                        "message": format!("Invalid interval: {}", interval)
                    }));
                }
            };

            if form.name.is_empty() || form.url.is_empty() {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "success": false,
                    "message": "Name and URL are required"
                }));
            }

            match sqlx::query!(
                "UPDATE monitors
                 SET name = ?, target_url = ?, type = ?, check_interval = ?, timeout = 10
                 WHERE id = ? AND user_id = ?",
                form.name,
                form.url,
                monitor_type,
                interval_num,
                id,
                user_id
            )
            .execute(db_pool.get_ref())
            .await
            {
                Ok(result) if result.rows_affected() > 0 => {
                    println!("✅ Monitor {} updated successfully", id);
                    HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "message": "Monitor updated successfully"
                    }))
                }
                Ok(_) => HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "message": "Monitor not found"
                })),
                Err(e) => {
                    eprintln!("❌ Failed to update monitor: {}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "success": false,
                        "message": format!("Database error: {}", e)
                    }))
                }
            }
        }
        _ => HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Unauthorized"
        })),
    }
}

// ==================== TOGGLE MONITOR ====================
async fn toggle_monitor(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    monitor_id: web::Path<i64>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let monitor_id_val = monitor_id.into_inner();

            match sqlx::query!(
                "SELECT is_active FROM monitors WHERE id = ? AND user_id = ?",
                monitor_id_val,
                user_id
            )
            .fetch_optional(db_pool.get_ref())
            .await
            {
                Ok(Some(monitor)) => {
                    let current_status = monitor.is_active.unwrap_or(true);
                    let new_status = !current_status;

                    match sqlx::query!(
                        "UPDATE monitors SET is_active = ? WHERE id = ?",
                        new_status,
                        monitor_id_val
                    )
                    .execute(db_pool.get_ref())
                    .await
                    {
                        Ok(_) => HttpResponse::Ok().finish(),
                        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
                    }
                }
                Ok(None) => HttpResponse::NotFound().body("Monitor not found"),
                Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
            }
        }
        _ => HttpResponse::Unauthorized().body("Unauthorized"),
    }
}

// ==================== DELETE MONITOR ====================
async fn delete_monitor(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    monitor_id: web::Path<i64>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let monitor_id_val = monitor_id.into_inner();

            let _ = sqlx::query!("DELETE FROM check_results WHERE monitor_id = ?", monitor_id_val)
                .execute(db_pool.get_ref()).await;
            let _ = sqlx::query!("DELETE FROM alerts WHERE monitor_id = ?", monitor_id_val)
                .execute(db_pool.get_ref()).await;

            match sqlx::query!(
                "DELETE FROM monitors WHERE id = ? AND user_id = ?",
                monitor_id_val,
                user_id
            )
            .execute(db_pool.get_ref())
            .await
            {
                Ok(result) if result.rows_affected() > 0 => HttpResponse::Ok().finish(),
                Ok(_) => HttpResponse::NotFound().body("Monitor not found"),
                Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
            }
        }
        _ => HttpResponse::Unauthorized().body("Unauthorized"),
    }
}

// ==================== RECENT MONITORS ====================
async fn recent_monitors(
    session: Session,
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let rows = sqlx::query(
                r#"SELECT
                    m.id,
                    m.name,
                    m.target_url,
                    m.type as monitor_type,
                    m.check_interval,
                    m.is_active,
                    m.timeout,
                    m.is_maintenance,
                    m.created_at,
                    cr.status as last_status,
                    cr.response_time as last_response_time,
                    cr.checked_at as last_checked
                FROM monitors m
                LEFT JOIN (
                    SELECT monitor_id, status, response_time, checked_at,
                           ROW_NUMBER() OVER (PARTITION BY monitor_id ORDER BY checked_at DESC) as rn
                    FROM check_results
                ) cr ON cr.monitor_id = m.id AND cr.rn = 1
                WHERE m.user_id = ?
                ORDER BY m.created_at DESC
                LIMIT 5"#
            )
            .bind(user_id)
            .fetch_all(db_pool.get_ref())
            .await;

            let monitors: Vec<MonitorRow> = match rows {
                Ok(rows) => {
                    let mut result = Vec::new();
                    for row in rows {
                        let last_checked: Option<String> = row.try_get("last_checked").ok();
                        let created_at: String = row.get("created_at");

                        result.push(MonitorRow {
                            id: row.get("id"),
                            name: row.get("name"),
                            target_url: row.get("target_url"),
                            monitor_type: row.get("monitor_type"),
                            check_interval: row.get("check_interval"),
                            is_active: row.get("is_active"),
                            last_status: row.get("last_status"),
                            last_response_time: row.get("last_response_time"),
                            last_checked: last_checked.and_then(|s| {
                                DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
                            }),
                            timeout: row.get("timeout"),
                            is_maintenance: row.get("is_maintenance"),
                            created_at: DateTime::parse_from_rfc3339(&created_at)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc)),
                        });
                    }
                    result
                }
                Err(e) => {
                    eprintln!("Database error: {:?}", e);
                    vec![]
                }
            };

            let mut html = String::new();

            if monitors.is_empty() {
                html.push_str(r##"<div class="p-8 text-center"><p class="text-slate-600">No monitors yet</p></div>"##);
            } else {
                for monitor in monitors {
                    let status_class = match monitor.last_status.as_deref() {
                        Some("up") => "status-up",
                        Some("down") => "status-down",
                        Some("slow") => "status-slow",
                        _ => "status-unknown",
                    };
                    let status_icon = match monitor.last_status.as_deref() {
                        Some("up") => "🟢",
                        Some("down") => "🔴",
                        Some("slow") => "🟡",
                        _ => "⚪",
                    };
                    let status_text = monitor.last_status.as_deref().unwrap_or("Unknown");
                    let last_checked_display = match monitor.last_checked {
                        Some(dt) => {
                            let diff = Utc::now() - dt;
                            if diff.num_minutes() < 1 { "Just now".to_string() }
                            else if diff.num_hours() < 1 { format!("{} min ago", diff.num_minutes()) }
                            else { format!("{} hours ago", diff.num_hours()) }
                        }
                        None => "Never".to_string(),
                    };

                    html.push_str(&format!(
                        r#"<div class="flex items-center justify-between p-4 border-b">
                            <div><span class="{} font-bold">{}{}</span> {}</div>
                            <div class="text-sm text-slate-500">{}</div>
                            <a href="/monitors" class="text-blue-600">View</a>
                        </div>"#,
                        status_class, status_icon, status_text, monitor.name, last_checked_display
                    ));
                }
            }

            HttpResponse::Ok().content_type("text/html").body(html)
        }
        _ => HttpResponse::Unauthorized().body("Unauthorized"),
    }
}

// ==================== STATUS UPDATES ====================
async fn monitor_status_updates(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    query: web::Query<StatusQuery>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let ids_vec: Vec<i64> = query.ids
                .split(',')
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.trim().parse().ok())
                .collect();

            if ids_vec.is_empty() {
                return HttpResponse::Ok().json(Vec::<MonitorStatus>::new());
            }

            let placeholders = ids_vec.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query_str = format!(
                r#"SELECT
                    m.id,
                    (SELECT cr.status FROM check_results cr
                     WHERE cr.monitor_id = m.id
                     ORDER BY cr.checked_at DESC LIMIT 1) as last_status,
                    (SELECT cr.response_time FROM check_results cr
                     WHERE cr.monitor_id = m.id
                     ORDER BY cr.checked_at DESC LIMIT 1) as last_response_time,
                    (SELECT cr.checked_at FROM check_results cr
                     WHERE cr.monitor_id = m.id
                     ORDER BY cr.checked_at DESC LIMIT 1) as last_checked
                   FROM monitors m
                   WHERE m.user_id = ? AND m.id IN ({})"#,
                placeholders
            );

            let mut query_builder = sqlx::query(&query_str).bind(user_id);
            for id in &ids_vec {
                query_builder = query_builder.bind(id);
            }

            let rows = query_builder.fetch_all(db_pool.get_ref()).await;
            let status_updates: Vec<MonitorStatus> = match rows {
                Ok(rows) => {
                    rows.iter().map(|row| {
                        let last_checked: Option<String> = row.try_get("last_checked").ok();
                        MonitorStatus {
                            id: row.get("id"),
                            status: row.get("last_status"),
                            response_time: row.get("last_response_time"),
                            last_checked: last_checked.and_then(|s| {
                                DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
                            }),
                        }
                    }).collect()
                }
                Err(_) => vec![],
            };

            HttpResponse::Ok().json(status_updates)
        }
        _ => HttpResponse::Unauthorized().finish(),
    }
}

// ==================== MONITORS DATA ====================
async fn monitors_data(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    query: web::Query<MonitorQuery>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            let page = query.page.unwrap_or(1);
            let limit = 10;
            let offset = (page - 1) * limit;
            let search = query.search.clone().unwrap_or_default();
            let status_filter = query.status.clone().unwrap_or_default();
            let type_filter = query.type_filter.clone().unwrap_or_default();

            let mut where_clauses = vec!["m.user_id = ?".to_string()];
            let mut params: Vec<String> = vec![user_id.to_string()];

            if !search.is_empty() {
                where_clauses.push("(m.name LIKE ? OR m.target_url LIKE ?)".to_string());
                params.push(format!("%{}%", search));
                params.push(format!("%{}%", search));
            }
            if !type_filter.is_empty() {
                where_clauses.push("m.type = ?".to_string());
                params.push(type_filter);
            }

            let where_sql = format!("WHERE {}", where_clauses.join(" AND "));
            let monitors_sql = format!(
                r#"SELECT
                    m.id,
                    m.name,
                    m.target_url,
                    m.type as monitor_type,
                    m.check_interval,
                    m.is_active,
                    m.timeout,
                    m.is_maintenance,
                    m.created_at,
                    (SELECT cr.status FROM check_results cr
                     WHERE cr.monitor_id = m.id
                     ORDER BY cr.checked_at DESC LIMIT 1) as last_status,
                    (SELECT cr.response_time FROM check_results cr
                     WHERE cr.monitor_id = m.id
                     ORDER BY cr.checked_at DESC LIMIT 1) as last_response_time,
                    (SELECT cr.checked_at FROM check_results cr
                     WHERE cr.monitor_id = m.id
                     ORDER BY cr.checked_at DESC LIMIT 1) as last_checked
                FROM monitors m
                {}
                ORDER BY m.created_at DESC
                LIMIT ? OFFSET ?"#,
                where_sql
            );

            let mut query_builder = sqlx::query(&monitors_sql);
            for param in &params {
                query_builder = query_builder.bind(param);
            }
            query_builder = query_builder.bind(limit).bind(offset);
            let rows = query_builder.fetch_all(db_pool.get_ref()).await;

            let mut monitors: Vec<MonitorRow> = Vec::new();
            if let Ok(rows) = rows {
                for row in rows {
                    let last_checked: Option<String> = row.try_get("last_checked").ok();
                    let created_at: String = row.get("created_at");

                    monitors.push(MonitorRow {
                        id: row.get("id"),
                        name: row.get("name"),
                        target_url: row.get("target_url"),
                        monitor_type: row.get("monitor_type"),
                        check_interval: row.get("check_interval"),
                        is_active: row.get("is_active"),
                        last_status: row.get("last_status"),
                        last_response_time: row.get("last_response_time"),
                        last_checked: last_checked.and_then(|s| {
                            DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
                        }),
                        timeout: row.get("timeout"),
                        is_maintenance: row.get("is_maintenance"),
                        created_at: DateTime::parse_from_rfc3339(&created_at)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc)),
                    });
                }
            }

            let filtered_monitors: Vec<MonitorRow> = if !status_filter.is_empty() {
                monitors.into_iter()
                    .filter(|m| match status_filter.as_str() {
                        "up" => m.last_status.as_deref() == Some("up"),
                        "down" => m.last_status.as_deref() == Some("down"),
                        "slow" => m.last_status.as_deref() == Some("slow"),
                        "unknown" => m.last_status.is_none(),
                        _ => true,
                    })
                    .collect()
            } else {
                monitors
            };

            let total = filtered_monitors.len() as i64;
            let total_pages = if total > 0 { (total as f64 / limit as f64).ceil() as i64 } else { 1 };

            let table_html = render_monitors_table(&filtered_monitors, page, total_pages, total);
            HttpResponse::Ok().content_type("text/html").body(table_html)
        }
        _ => HttpResponse::Unauthorized().finish(),
    }
}

// ==================== RENDER TABLE ====================
fn render_monitors_table(monitors: &[MonitorRow], current_page: i64, total_pages: i64, total_records: i64) -> String {
    let mut html = String::new();

    if monitors.is_empty() {
        html.push_str(r##"<div class="p-12 text-center"><h3 class="text-lg font-medium">No monitors found</h3></div>"##);
        return html;
    }

    html.push_str(r#"<div class="overflow-x-auto"><table class="w-full"><thead><tr class="bg-slate-50">"#);
    html.push_str(r#"<th class="text-left py-4 px-6">Name</th><th class="text-left py-4 px-6">URL</th>"#);
    html.push_str(r#"<th class="text-left py-4 px-6">Type</th><th class="text-left py-4 px-6">Interval</th>"#);
    html.push_str(r#"<th class="text-left py-4 px-6">Status</th><th class="text-left py-4 px-6">Response</th>"#);
    html.push_str(r#"<th class="text-left py-4 px-6">Last Check</th><th class="text-left py-4 px-6">Active</th>"#);
    html.push_str(r#"<th class="text-left py-4 px-6">Actions</th></tr></thead><tbody>"#);

    for monitor in monitors {
        let status_class = match monitor.last_status.as_deref() {
            Some("up") => "status-up",
            Some("down") => "status-down",
            Some("slow") => "status-slow",
            _ => "status-unknown",
        };
        let status_dot_class = match monitor.last_status.as_deref() {
            Some("up") => "up",
            Some("down") => "down",
            Some("slow") => "slow",
            _ => "unknown",
        };
        let status_text = monitor.last_status.as_deref().unwrap_or("Unknown");
        let response_time = monitor.last_response_time.map(|rt| format!("{}ms", rt)).unwrap_or_else(|| "N/A".to_string());

        let last_checked_display = match monitor.last_checked {
            Some(dt) => {
                let diff = Utc::now() - dt;
                if diff.num_minutes() < 1 { "Just now".to_string() }
                else if diff.num_hours() < 1 { format!("{} min ago", diff.num_minutes()) }
                else { format!("{} hours ago", diff.num_hours()) }
            }
            None => "Never".to_string(),
        };

        let not_active_str = if !monitor.is_active { "true" } else { "false" };

        html.push_str(&format!(
            r#"<tr id="monitor-row-{}">
                <td class="py-4 px-6"><div class="font-medium">{}</div><div class="text-sm">ID: {}</div></td>
                <td class="py-4 px-6"><a href="{}" target="_blank" class="text-blue-600">{}</a></td>
                <td class="py-4 px-6"><span class="px-3 py-1 bg-slate-100 rounded-full text-sm">{}</span></td>
                <td class="py-4 px-6">{}s</td>
                <td class="py-4 px-6"><span class="{} flex items-center"><span class="status-dot {}"></span>{}</span></td>
                <td class="py-4 px-6 font-mono" id="response-{}">{}</td>
                <td class="py-4 px-6 text-sm" id="last-checked-{}">{}</td>
                <td class="py-4 px-6">
                    <label class="switch"><input type="checkbox" {} onchange="toggleMonitorStatus({}, {})"><span class="slider round"></span></label>
                </td>
                <td class="py-4 px-6">
                    <div class="flex space-x-2 items-center">
                        <button onclick="copyPublicLink({})"
                                class="text-blue-600 hover:text-blue-800 text-sm mr-2" title="Copy Public Link">
                            🔗
                        </button>
                        <button onclick="editMonitor({})" class="p-1 text-blue-600 hover:text-blue-800" title="Edit">
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/>
                            </svg>
                        </button>
                        <button onclick="deleteMonitor({})" class="p-1 text-red-600 hover:text-red-800" title="Delete">
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                            </svg>
                        </button>
                    </div>
                </td>
            </tr>"#,
            monitor.id, monitor.name, monitor.id, monitor.target_url, monitor.target_url,
            monitor.monitor_type.to_uppercase(), monitor.check_interval,
            status_class, status_dot_class, status_text,
            monitor.id, response_time, monitor.id, last_checked_display,
            if monitor.is_active { "checked" } else { "" }, monitor.id, not_active_str,
            monitor.id, monitor.id, monitor.id
        ));
    }

    html.push_str(r#"</tbody></table></div>"#);

    if total_pages > 1 {
        html.push_str("<div class=\"p-4 border-t border-slate-200\"><div class=\"flex flex-col md:flex-row justify-between items-center\">");
        html.push_str("<div class=\"text-slate-600 mb-4 md:mb-0\">");
        html.push_str(&format!("Showing <span class=\"font-semibold\">{}</span> to <span class=\"font-semibold\">{}</span> of <span class=\"font-semibold\">{}</span> monitors",
            ((current_page - 1) * 10 + 1).min(total_records),
            (current_page * 10).min(total_records),
            total_records
        ));
        html.push_str("</div><div class=\"flex space-x-1\">");

        if current_page > 1 {
            html.push_str(&format!("<button onclick=\"loadPage({})\" class=\"px-3 py-1 rounded-lg border border-slate-300 text-slate-700 hover:bg-slate-50\">« Previous</button>", current_page - 1));
        }

        let start_page = (current_page - 2).max(1);
        let end_page = (current_page + 2).min(total_pages);

        for page_num in start_page..=end_page {
            if page_num == current_page {
                html.push_str(&format!("<button class=\"px-3 py-1 rounded-lg border border-blue-500 bg-blue-500 text-white\">{}</button>", page_num));
            } else {
                html.push_str(&format!("<button onclick=\"loadPage({})\" class=\"px-3 py-1 rounded-lg border border-slate-300 text-slate-700 hover:bg-slate-50\">{}</button>", page_num, page_num));
            }
        }

        if current_page < total_pages {
            html.push_str(&format!("<button onclick=\"loadPage({})\" class=\"px-3 py-1 rounded-lg border border-slate-300 text-slate-700 hover:bg-slate-50\">Next »</button>", current_page + 1));
        }

        html.push_str("</div></div></div>");
    }

    html
}
