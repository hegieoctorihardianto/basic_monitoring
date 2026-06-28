// src/dashboard.rs
use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::SqlitePool;

// INCLUDE SEPERTI PHP
use crate::navbar;
use crate::sidebar;
use crate::footer;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/dashboard")
            .route("", web::get().to(dashboard_page))
            .route("/data", web::get().to(dashboard_data))
    );
}

async fn dashboard_page(session: Session, db_pool: web::Data<SqlitePool>) -> impl Responder {
    // Check if user is logged in
    match session.get::<String>("email") {
        Ok(Some(email)) => {
            render_dashboard(&email, &db_pool).await
        }
        _ => {
            // Redirect to login
            HttpResponse::Found()
                .append_header(("Location", "/"))
                .finish()
        }
    }
}

async fn render_dashboard(email: &str, db_pool: &SqlitePool) -> HttpResponse {
    // MULAI BUILD HTML SEPERTI PHP
    let mut html = String::new();

    // Get user_id from email - Use fetch_optional to avoid errors
    let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE email = ?", email)
        .fetch_optional(db_pool)
        .await
        .unwrap_or(None);

    // Jika user tidak ditemukan, redirect ke login
    if user_id.is_none() {
        return HttpResponse::Found()
            .append_header(("Location", "/"))
            .finish();
    }

    // DOCTYPE DAN HEAD
    html.push_str(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Dashboard - Basic Monitoring</title>
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

        .stat-card {
            transition: transform 0.2s ease;
        }

        .stat-card:hover {
            transform: translateY(-2px);
        }

        .status-online {
            background-color: #10b981;
            box-shadow: 0 0 0 4px rgba(16, 185, 129, 0.2);
        }

        .status-warning {
            background-color: #f59e0b;
            box-shadow: 0 0 0 4px rgba(245, 158, 11, 0.2);
        }

        .status-offline {
            background-color: #ef4444;
            box-shadow: 0 0 0 4px rgba(239, 68, 68, 0.2);
        }

        /* Modal styles */
        .modal-overlay {
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background-color: rgba(0, 0, 0, 0.5);
            z-index: 50;
            align-items: center;
            justify-content: center;
            padding: 1rem;
        }

        .modal-overlay.active {
            display: flex;
        }

        .modal-content {
            background-color: white;
            border-radius: 1rem;
            width: 100%;
            max-width: 42rem;
            max-height: 90vh;
            overflow: auto;
        }

        /* Real-time indicator */
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

        /* Status dots */
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

        /* Loading animation */
        .loading {
            position: relative;
        }

        .loading::after {
            content: "";
            position: absolute;
            top: 50%;
            left: 50%;
            width: 20px;
            height: 20px;
            margin: -10px 0 0 -10px;
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

    // KONTEN UTAMA DASHBOARD
    html.push_str(&format!(r##"
        <!-- Main Content -->
        <div class="flex-1 p-6">
            <!-- Welcome Card -->
            <div class="bg-gradient-to-r from-blue-500 to-purple-600 rounded-2xl p-8 text-white mb-8">
                <div class="flex justify-between items-start">
                    <div>
                        <h1 class="text-3xl font-bold mb-4">Welcome to Your Dashboard</h1>
                        <p class="text-blue-100 mb-6">Monitor your websites in real-time. Get instant alerts when something goes wrong.</p>
                    </div>
                    <div class="flex items-center">
                        <div id="dashboard-realtime-status" class="realtime-indicator hidden">
                            <div class="animate-spin rounded-full h-3 w-3 border-t-2 border-b-2 border-green-500 mr-1"></div>
                            <span>Live Updates</span>
                        </div>
                    </div>
                </div>
                <button onclick="openAddMonitorModal()"
                        class="inline-flex items-center bg-white text-blue-600 font-semibold px-6 py-3 rounded-lg hover:bg-gray-100 cursor-pointer">
                    <span>➕ Add New Monitor</span>
                    <svg class="w-5 h-5 ml-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
                    </svg>
                </button>
            </div>

            <!-- Stats Overview -->
            <div class="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
                <div class="stat-card bg-white rounded-xl shadow p-6">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-slate-500 text-sm">Total Monitors</p>
                            <p class="text-3xl font-bold" id="total-monitors-count">0</p>
                        </div>
                        <div class="w-12 h-12 bg-blue-100 rounded-lg flex items-center justify-center">
                            <span class="text-2xl text-blue-600">🌐</span>
                        </div>
                    </div>
                </div>

                <div class="stat-card bg-white rounded-xl shadow p-6">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-slate-500 text-sm">Uptime</p>
                            <p class="text-3xl font-bold" id="uptime-percentage">99.9%</p>
                        </div>
                        <div class="w-12 h-12 bg-green-100 rounded-lg flex items-center justify-center">
                            <span class="text-2xl text-green-600">📈</span>
                        </div>
                    </div>
                </div>

                <div class="stat-card bg-white rounded-xl shadow p-6">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-slate-500 text-sm">Active Alerts</p>
                            <p class="text-3xl font-bold" id="active-alerts-count">0</p>
                        </div>
                        <div class="w-12 h-12 bg-yellow-100 rounded-lg flex items-center justify-center">
                            <span class="text-2xl text-yellow-600">🔔</span>
                        </div>
                    </div>
                </div>

                <div class="stat-card bg-white rounded-xl shadow p-6">
                    <div class="flex items-center justify-between">
                        <div>
                            <p class="text-slate-500 text-sm">Avg Response</p>
                            <p class="text-3xl font-bold" id="avg-response-time">124ms</p>
                        </div>
                        <div class="w-12 h-12 bg-purple-100 rounded-lg flex items-center justify-center">
                            <span class="text-2xl text-purple-600">⚡</span>
                        </div>
                    </div>
                </div>
            </div>

            <!-- SSL Certificate Stats -->
            <div class="bg-white rounded-xl shadow p-6 mb-8">
                <div class="flex items-center justify-between mb-4">
                    <h2 class="text-xl font-bold">🔒 SSL Certificate Status</h2>
                    <a href="/ssl-monitors" class="text-blue-600 hover:text-blue-800 text-sm">View All →</a>
                </div>
                <div class="grid grid-cols-2 md:grid-cols-5 gap-4">
                    <div class="text-center p-3 bg-slate-50 rounded-lg">
                        <div class="text-2xl font-bold text-slate-700" id="ssl-total">0</div>
                        <div class="text-xs text-slate-500">Total</div>
                    </div>
                    <div class="text-center p-3 bg-green-50 rounded-lg">
                        <div class="text-2xl font-bold text-green-600" id="ssl-valid">0</div>
                        <div class="text-xs text-green-600">Valid</div>
                    </div>
                    <div class="text-center p-3 bg-yellow-50 rounded-lg">
                        <div class="text-2xl font-bold text-yellow-600" id="ssl-expiring">0</div>
                        <div class="text-xs text-yellow-600">Expiring Soon</div>
                    </div>
                    <div class="text-center p-3 bg-red-50 rounded-lg">
                        <div class="text-2xl font-bold text-red-600" id="ssl-expired">0</div>
                        <div class="text-xs text-red-600">Expired</div>
                    </div>
                    <div class="text-center p-3 bg-gray-50 rounded-lg">
                        <div class="text-2xl font-bold text-gray-600" id="ssl-unknown">0</div>
                        <div class="text-xs text-gray-500">Unknown</div>
                    </div>
                </div>
                <div class="mt-4 text-xs text-slate-400 text-center">
                    ⚠️ Certificates expiring in less than 7 days will be marked as "Expiring Soon"
                </div>
            </div>

            <!-- Recent Activity & Quick Actions -->
            <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
                <!-- Recent Monitors -->
                <div class="lg:col-span-2">
                    <div class="bg-white rounded-xl shadow p-6">
                        <div class="flex justify-between items-center mb-6">
                            <h2 class="text-xl font-bold">Recent Monitors</h2>
                            <div class="flex items-center space-x-2">
                                <span class="text-sm text-slate-500" id="last-update-time">
                                    Updated: <span id="update-time">Just now</span>
                                </span>
                                <a href="/monitors" class="text-blue-600 hover:text-blue-800">View All</a>
                            </div>
                        </div>

                        <div id="recent-monitors" class="space-y-4">
                            <!-- Recent monitors akan di-load via JavaScript -->
                            <div class="p-12 text-center">
                                <div class="inline-block animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500"></div>
                                <p class="mt-2 text-slate-500">Loading monitors...</p>
                            </div>
                        </div>

                        <div class="mt-6">
                            <button onclick="openAddMonitorModal()"
                                    class="block w-full text-center border-2 border-dashed border-slate-300 text-slate-600 p-4 rounded-lg hover:border-blue-500 hover:text-blue-600 cursor-pointer">
                                + Add New Website Monitor
                            </button>
                        </div>
                    </div>
                </div>

                <!-- Quick Actions -->
                <div>
                    <div class="bg-white rounded-xl shadow p-6 mb-6">
                        <h2 class="text-xl font-bold mb-6">Quick Actions</h2>
                        <div class="space-y-4">
                            <button onclick="openAddMonitorModal()"
                                    class="flex items-center space-x-3 p-4 border border-slate-200 rounded-lg hover:bg-slate-50 w-full text-left">
                                <div class="w-10 h-10 bg-blue-100 rounded-lg flex items-center justify-center">
                                    <span class="text-blue-600">➕</span>
                                </div>
                                <div>
                                    <p class="font-semibold">Add Monitor</p>
                                    <p class="text-sm text-slate-500">Monitor new website</p>
                                </div>
                            </button>

                            <a href="/ssl-monitors" class="flex items-center space-x-3 p-4 border border-slate-200 rounded-lg hover:bg-slate-50">
                                <div class="w-10 h-10 bg-purple-100 rounded-lg flex items-center justify-center">
                                    <span class="text-purple-600">🔒</span>
                                </div>
                                <div>
                                    <p class="font-semibold">SSL Monitors</p>
                                    <p class="text-sm text-slate-500">Monitor SSL certificates</p>
                                </div>
                            </a>

                            <a href="/alerts" class="flex items-center space-x-3 p-4 border border-slate-200 rounded-lg hover:bg-slate-50">
                                <div class="w-10 h-10 bg-green-100 rounded-lg flex items-center justify-center">
                                    <span class="text-green-600">🔔</span>
                                </div>
                                <div>
                                    <p class="font-semibold">Setup Alerts</p>
                                    <p class="text-sm text-slate-500">Configure notifications</p>
                                </div>
                            </a>

                            <a href="/reports" class="flex items-center space-x-3 p-4 border border-slate-200 rounded-lg hover:bg-slate-50">
                                <div class="w-10 h-10 bg-purple-100 rounded-lg flex items-center justify-center">
                                    <span class="text-purple-600">📈</span>
                                </div>
                                <div>
                                    <p class="font-semibold">View Reports</p>
                                    <p class="text-sm text-slate-500">Performance analytics</p>
                                </div>
                            </a>
                        </div>
                    </div>

                    <!-- System Status -->
                    <div class="bg-gradient-to-r from-slate-800 to-slate-900 rounded-xl shadow p-6 text-white" style="color:black;">
                        <h2 class="text-xl font-bold mb-4" style="color:black;">System Status</h2>
                        <div class="space-y-4">
                            <div class="flex items-center justify-between">
                                <span>Database</span>
                                <span class="px-3 py-1 bg-green-500/20 text-green-400 rounded-full text-sm">Online</span>
                            </div>
                            <div class="flex items-center justify-between">
                                <span>Monitoring Workers</span>
                                <span class="px-3 py-1 bg-green-500/20 text-green-400 rounded-full text-sm">Active</span>
                            </div>
                            <div class="flex items-center justify-between">
                                <span>SSL Worker</span>
                                <span class="px-3 py-1 bg-green-500/20 text-green-400 rounded-full text-sm">Active</span>
                            </div>
                            <div class="flex items-center justify-between">
                                <span>Last Check</span>
                                <span class="text-slate-300" id="system-last-check">Just now</span>
                            </div>
                            <div class="flex items-center justify-between">
                                <span>Real-time Updates</span>
                                <span class="px-3 py-1 bg-green-500/20 text-green-400 rounded-full text-sm" id="realtime-status-indicator">Active</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>"##));

    // MODAL ADD MONITOR
    html.push_str(r##"
    <!-- Modal Add Monitor -->
    <div id="add-monitor-modal" class="modal-overlay">
        <div class="modal-content">
            <div class="p-6 border-b border-slate-200">
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
                <input type="hidden" name="form_type" value="multiple">
                <div id="monitor-fields">
                    <!-- Monitor 1 -->
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
                                <input type="text" name="names[]"
                                       class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                                       placeholder="My Website" required>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">URL *</label>
                                <input type="url" name="urls[]"
                                       class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                                       placeholder="https://example.com" required>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">Type</label>
                                <select name="types[]" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                                    <option value="http">HTTP</option>
                                    <option value="https">HTTPS</option>
                                    <option value="ping">PING</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">Check Interval (seconds)</label>
                                <select name="intervals[]" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
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

    <script>
        let monitorCounter = 1;
        let dashboardPollingInterval = null;
        let isDashboardPollingActive = true;
        let lastDashboardUpdateTime = new Date();
        let recentMonitorIds = new Set(); // Track recent monitor IDs

        function openAddMonitorModal() {
            console.log('Opening modal from dashboard...');
            const modal = document.getElementById('add-monitor-modal');
            if (modal) {
                modal.classList.add('active');
                // Reset form
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
                                <input type="text" name="names[]"
                                       class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                                       placeholder="My Website" required>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">URL *</label>
                                <input type="url" name="urls[]"
                                       class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                                       placeholder="https://example.com" required>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">Type</label>
                                <select name="types[]" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                                    <option value="http">HTTP</option>
                                    <option value="https">HTTPS</option>
                                    <option value="ping">PING</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-slate-700 font-medium mb-2">Check Interval (seconds)</label>
                                <select name="intervals[]" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
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
            } else {
                console.error('Modal element not found!');
            }
        }

        function closeAddMonitorModal() {
            const modal = document.getElementById('add-monitor-modal');
            if (modal) {
                modal.classList.remove('active');
            }
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
                        <input type="text" name="names[]"
                               class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                               placeholder="My Website" required>
                    </div>
                    <div>
                        <label class="block text-slate-700 font-medium mb-2">URL *</label>
                        <input type="url" name="urls[]"
                               class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                               placeholder="https://example.com" required>
                    </div>
                    <div>
                        <label class="block text-slate-700 font-medium mb-2">Type</label>
                        <select name="types[]" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
                            <option value="http">HTTP</option>
                            <option value="https">HTTPS</option>
                            <option value="ping">PING</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-slate-700 font-medium mb-2">Check Interval (seconds)</label>
                        <select name="intervals[]" class="w-full px-4 py-2 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200">
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
                // Update numbering
                const fields = document.querySelectorAll('.monitor-field');
                fields.forEach((field, index) => {
                    const title = field.querySelector('.text-slate-700.font-medium');
                    if (title) {
                        title.textContent = 'Monitor #' + (index + 1);
                    }
                });
                monitorCounter = fields.length;
            }
        }

        function submitMonitorForm() {
            const form = document.getElementById('add-monitor-form');
            if (!form) {
                console.error('Form not found');
                return;
            }

            // Collect data from form
            const monitors = [];
            const monitorFields = form.querySelectorAll('.monitor-field');

            monitorFields.forEach((field, index) => {
                const nameInput = field.querySelector('input[name="names[]"]');
                const urlInput = field.querySelector('input[name="urls[]"]');
                const typeSelect = field.querySelector('select[name="types[]"]');
                const intervalSelect = field.querySelector('select[name="intervals[]"]');

                if (nameInput && nameInput.value.trim() && urlInput && urlInput.value.trim()) {
                    monitors.push({
                        name: nameInput.value.trim(),
                        url: urlInput.value.trim(),
                        type: typeSelect ? typeSelect.value : 'http',
                        interval: intervalSelect ? intervalSelect.value : '60'
                    });
                }
            });

            if (monitors.length === 0) {
                Swal.fire({
                    icon: 'error',
                    title: 'Error',
                    text: 'Please enter at least one monitor with name and URL',
                });
                return;
            }

            console.log('Submitting monitors:', monitors);

            // Send data to server
            fetch('/monitors', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    monitors: monitors
                })
            })
            .then(response => {
                console.log('Response status:', response.status);

                if (response.redirected) {
                    window.location.href = response.url;
                    return;
                }

                return response.text().then(text => {
                    console.log('Response text:', text);

                    if (response.ok) {
                        closeAddMonitorModal();
                        Swal.fire({
                            icon: 'success',
                            title: 'Success!',
                            text: `${monitors.length} monitor(s) added successfully`,
                            timer: 2000,
                            showConfirmButton: false
                        }).then(() => {
                            loadDashboardData();
                            loadRecentMonitors();
                        });
                    } else {
                        Swal.fire({
                            icon: 'error',
                            title: 'Error',
                            html: text,
                        });
                    }
                });
            })
            .catch(error => {
                console.error('Network error:', error);
                Swal.fire({
                    icon: 'error',
                    title: 'Network Error',
                    text: 'Unable to connect to server: ' + error.message,
                });
            });
        }

        // Update last update time
        function updateLastUpdateTime() {
            lastDashboardUpdateTime = new Date();
            const timeStr = formatTime(lastDashboardUpdateTime);
            const updateTimeElement = document.getElementById('update-time');
            const systemLastCheckElement = document.getElementById('system-last-check');

            if (updateTimeElement) {
                updateTimeElement.textContent = timeStr;
            }
            if (systemLastCheckElement) {
                systemLastCheckElement.textContent = timeStr;
            }
        }

        function formatTime(date) {
            const now = new Date();
            const diffMs = now - date;
            const diffSec = Math.floor(diffMs / 1000);
            const diffMin = Math.floor(diffSec / 60);

            if (diffSec < 10) return 'Just now';
            if (diffSec < 60) return `${diffSec} seconds ago`;
            if (diffMin < 60) return `${diffMin} minute${diffMin > 1 ? 's' : ''} ago`;

            return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
        }

        // Start real-time polling for dashboard
        function startDashboardPolling() {
            if (dashboardPollingInterval) {
                clearInterval(dashboardPollingInterval);
            }

            dashboardPollingInterval = setInterval(() => {
                if (isDashboardPollingActive && recentMonitorIds.size > 0) {
                    fetchDashboardStatusUpdates();
                }
            }, 15000);

            const indicator = document.getElementById('dashboard-realtime-status');
            const statusIndicator = document.getElementById('realtime-status-indicator');

            if (indicator) {
                indicator.classList.remove('hidden');
                indicator.classList.add('blinking');
            }
            if (statusIndicator) {
                statusIndicator.textContent = 'Active';
                statusIndicator.className = 'px-3 py-1 bg-green-500/20 text-green-400 rounded-full text-sm';
            }

            console.log('Dashboard real-time polling started');
        }

        function stopDashboardPolling() {
            if (dashboardPollingInterval) {
                clearInterval(dashboardPollingInterval);
                dashboardPollingInterval = null;
            }

            const indicator = document.getElementById('dashboard-realtime-status');
            const statusIndicator = document.getElementById('realtime-status-indicator');

            if (indicator) {
                indicator.classList.add('hidden');
                indicator.classList.remove('blinking');
            }
            if (statusIndicator) {
                statusIndicator.textContent = 'Paused';
                statusIndicator.className = 'px-3 py-1 bg-yellow-500/20 text-yellow-400 rounded-full text-sm';
            }

            console.log('Dashboard real-time polling stopped');
        }

        function fetchDashboardStatusUpdates() {
            if (recentMonitorIds.size === 0) return;

            const idsString = Array.from(recentMonitorIds).join(',');

            fetch(`/monitors/status-updates?ids=${idsString}`)
                .then(response => {
                    if (!response.ok) {
                        throw new Error(`HTTP error! status: ${response.status}`);
                    }
                    return response.json();
                })
                .then(data => {
                    updateRecentMonitorsStatus(data);
                    updateLastUpdateTime();
                    updateDashboardStats(data);
                })
                .catch(error => {
                    console.error('Error fetching dashboard status updates:', error);
                });
        }

        function updateRecentMonitorsStatus(statuses) {
            if (!statuses || !Array.isArray(statuses)) return;

            statuses.forEach(status => {
                const monitorElement = document.querySelector(`.recent-monitor[data-monitor-id="${status.id}"]`);
                if (monitorElement) {
                    const statusDot = monitorElement.querySelector('.status-dot');
                    if (statusDot) {
                        statusDot.className = 'status-dot ' + getStatusDotClass(status.status);
                    }

                    const statusText = monitorElement.querySelector('.status-text');
                    if (statusText) {
                        statusText.textContent = getStatusText(status.status);
                        statusText.className = 'status-text ' + getStatusTextClass(status.status);
                    }

                    const responseTime = monitorElement.querySelector('.response-time');
                    if (responseTime && status.response_time !== null && status.response_time !== undefined) {
                        responseTime.textContent = `${status.response_time}ms`;
                    }

                    const lastChecked = monitorElement.querySelector('.last-checked');
                    if (lastChecked && status.last_checked) {
                        try {
                            const checkedDate = new Date(status.last_checked);
                            if (!isNaN(checkedDate.getTime())) {
                                lastChecked.textContent = formatTime(checkedDate);
                            }
                        } catch (e) {
                            console.error('Error parsing date:', e);
                        }
                    }
                }
            });
        }

        function updateDashboardStats(statuses) {
            if (!statuses || !Array.isArray(statuses) || statuses.length === 0) return;

            let upCount = 0;
            let downCount = 0;
            let totalResponseTime = 0;
            let responseCount = 0;

            statuses.forEach(status => {
                if (status.status === 'up') upCount++;
                if (status.status === 'down') downCount++;
                if (status.response_time && status.response_time > 0) {
                    totalResponseTime += status.response_time;
                    responseCount++;
                }
            });

            const totalMonitors = statuses.length;
            const uptimePercentage = totalMonitors > 0 ? ((upCount / totalMonitors) * 100).toFixed(1) : 0;
            document.getElementById('uptime-percentage').textContent = uptimePercentage + '%';
            document.getElementById('active-alerts-count').textContent = downCount;

            if (responseCount > 0) {
                const avgResponse = Math.round(totalResponseTime / responseCount);
                document.getElementById('avg-response-time').textContent = avgResponse + 'ms';
            }
        }

        function updateSslStats(sslStats) {
            if (sslStats) {
                document.getElementById('ssl-total').textContent = sslStats.total || 0;
                document.getElementById('ssl-valid').textContent = sslStats.valid || 0;
                document.getElementById('ssl-expiring').textContent = sslStats.expiring || 0;
                document.getElementById('ssl-expired').textContent = sslStats.expired || 0;
                document.getElementById('ssl-unknown').textContent = sslStats.unknown || 0;
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

        function getStatusTextClass(status) {
            switch(status) {
                case 'up': return 'text-green-600';
                case 'down': return 'text-red-600';
                case 'slow': return 'text-yellow-600';
                default: return 'text-gray-600';
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

        function loadDashboardData() {
            fetch('/dashboard/data')
                .then(response => response.json())
                .then(data => {
                    console.log('Dashboard data:', data);
                    if (data.status === 'success') {
                        document.getElementById('total-monitors-count').textContent = data.total_monitors;
                        if (data.uptime_percentage) {
                            document.getElementById('uptime-percentage').textContent = data.uptime_percentage + '%';
                        }
                        if (data.avg_response_time) {
                            document.getElementById('avg-response-time').textContent = Math.round(data.avg_response_time) + 'ms';
                        }
                        if (data.ssl_stats) {
                            updateSslStats(data.ssl_stats);
                        }
                    }
                })
                .catch(error => {
                    console.error('Error loading dashboard data:', error);
                });
        }

        function loadRecentMonitors() {
            fetch('/monitors/recent')
                .then(response => response.text())
                .then(html => {
                    document.getElementById('recent-monitors').innerHTML = html;
                    extractRecentMonitorIds();
                    updateLastUpdateTime();
                })
                .catch(error => {
                    console.error('Error loading recent monitors:', error);
                    document.getElementById('recent-monitors').innerHTML =
                        '<p class="text-slate-500 text-center">Failed to load monitors</p>';
                });
        }

        function extractRecentMonitorIds() {
            recentMonitorIds.clear();
            const monitorElements = document.querySelectorAll('.recent-monitor');
            monitorElements.forEach(element => {
                const monitorId = element.getAttribute('data-monitor-id');
                if (monitorId) {
                    recentMonitorIds.add(parseInt(monitorId));
                }
            });
            console.log(`Extracted ${recentMonitorIds.size} recent monitor IDs for real-time updates`);
        }

        document.addEventListener('visibilitychange', function() {
            if (document.hidden) {
                isDashboardPollingActive = false;
                console.log('Dashboard page hidden, polling paused');
            } else {
                isDashboardPollingActive = true;
                console.log('Dashboard page visible, polling resumed');
            }
        });

        document.addEventListener('keydown', function(e) {
            if (e.key === 'Escape') closeAddMonitorModal();
        });

        const modal = document.getElementById('add-monitor-modal');
        if (modal) {
            modal.addEventListener('click', function(e) {
                if (e.target === this) closeAddMonitorModal();
            });
        }

        document.addEventListener('DOMContentLoaded', function() {
            console.log('Dashboard loaded');
            loadDashboardData();
            loadRecentMonitors();

            setTimeout(() => {
                startDashboardPolling();
            }, 3000);

            document.querySelectorAll('button[onclick*="openAddMonitorModal"]').forEach(btn => {
                btn.onclick = openAddMonitorModal;
            });

            window.addEventListener('beforeunload', function() {
                stopDashboardPolling();
            });
        });
    </script>
"##);

    // INCLUDE FOOTER
    html.push_str(&footer::render());

    // TUTUP BODY DAN HTML
    html.push_str("</body></html>");

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

async fn dashboard_data(
    session: Session,
    db_pool: web::Data<SqlitePool>,
) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => {
            // Get user_id from email
            match sqlx::query_scalar!("SELECT id FROM users WHERE email = ?", email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    // Get monitor count for this user
                    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM monitors WHERE user_id = ?")
                        .bind(user_id)
                        .fetch_one(db_pool.get_ref())
                        .await
                        .unwrap_or((0,));

                    // Get uptime stats
                    let up_count: (i64,) = sqlx::query_as(
                        "SELECT COUNT(DISTINCT m.id) FROM monitors m
                         LEFT JOIN check_results cr ON m.id = cr.monitor_id
                         WHERE m.user_id = ? AND cr.status = 'up'
                         AND cr.checked_at >= datetime('now', '-24 hours')"
                    )
                    .bind(user_id)
                    .fetch_one(db_pool.get_ref())
                    .await
                    .unwrap_or((0,));

                    let total_checks: (i64,) = sqlx::query_as(
                        "SELECT COUNT(DISTINCT cr.id) FROM monitors m
                         LEFT JOIN check_results cr ON m.id = cr.monitor_id
                         WHERE m.user_id = ? AND cr.checked_at >= datetime('now', '-24 hours')"
                    )
                    .bind(user_id)
                    .fetch_one(db_pool.get_ref())
                    .await
                    .unwrap_or((0,));

                    let uptime_percentage = if total_checks.0 > 0 {
                        (up_count.0 as f64 / total_checks.0 as f64 * 100.0).round()
                    } else {
                        100.0
                    };

                    // Get average response time
                    let avg_response: (Option<f64>,) = sqlx::query_as(
                        "SELECT AVG(cr.response_time) FROM monitors m
                         LEFT JOIN check_results cr ON m.id = cr.monitor_id
                         WHERE m.user_id = ? AND cr.checked_at >= datetime('now', '-1 hour')"
                    )
                    .bind(user_id)
                    .fetch_one(db_pool.get_ref())
                    .await
                    .unwrap_or((None,));

                    // ========== SSL STATS ==========
                    // Get SSL monitor stats
                    let ssl_total: (i64,) = sqlx::query_as(
                        "SELECT COUNT(*) FROM ssl_monitors WHERE user_id = ?"
                    )
                    .bind(user_id)
                    .fetch_one(db_pool.get_ref())
                    .await
                    .unwrap_or((0,));

                    let ssl_valid: (i64,) = sqlx::query_as(
                        "SELECT COUNT(*) FROM ssl_monitors
                         WHERE user_id = ? AND expiry_date > CURRENT_TIMESTAMP
                         AND julianday(expiry_date) - julianday('now') > 7"
                    )
                    .bind(user_id)
                    .fetch_one(db_pool.get_ref())
                    .await
                    .unwrap_or((0,));

                    let ssl_expiring: (i64,) = sqlx::query_as(
                        "SELECT COUNT(*) FROM ssl_monitors
                         WHERE user_id = ? AND expiry_date > CURRENT_TIMESTAMP
                         AND julianday(expiry_date) - julianday('now') <= 7"
                    )
                    .bind(user_id)
                    .fetch_one(db_pool.get_ref())
                    .await
                    .unwrap_or((0,));

                    let ssl_expired: (i64,) = sqlx::query_as(
                        "SELECT COUNT(*) FROM ssl_monitors
                         WHERE user_id = ? AND expiry_date <= CURRENT_TIMESTAMP"
                    )
                    .bind(user_id)
                    .fetch_one(db_pool.get_ref())
                    .await
                    .unwrap_or((0,));

                    let ssl_unknown: (i64,) = sqlx::query_as(
                        "SELECT COUNT(*) FROM ssl_monitors
                         WHERE user_id = ? AND expiry_date IS NULL"
                    )
                    .bind(user_id)
                    .fetch_one(db_pool.get_ref())
                    .await
                    .unwrap_or((0,));

                    HttpResponse::Ok().json(serde_json::json!({
                        "total_monitors": count.0,
                        "uptime_percentage": uptime_percentage,
                        "avg_response_time": avg_response.0.unwrap_or(0.0),
                        "ssl_stats": {
                            "total": ssl_total.0,
                            "valid": ssl_valid.0,
                            "expiring": ssl_expiring.0,
                            "expired": ssl_expired.0,
                            "unknown": ssl_unknown.0
                        },
                        "status": "success"
                    }))
                }
                Ok(None) => {
                    HttpResponse::Unauthorized().json(serde_json::json!({
                        "status": "error",
                        "message": "User not found"
                    }))
                }
                Err(e) => {
                    eprintln!("Database error in dashboard_data: {:?}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "status": "error",
                        "message": "Database error"
                    }))
                }
            }
        }
        _ => HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}
