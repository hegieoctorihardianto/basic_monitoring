// src/reports.rs
use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::{SqlitePool, Row};
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration, NaiveDateTime};

// INCLUDE SEPERTI PHP
use crate::navbar;
use crate::sidebar;
use crate::footer;

#[derive(Deserialize)]
pub struct ReportQuery {
    pub period: Option<String>,
    pub monitor_id: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub format: Option<String>,
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/reports")
            .route("", web::get().to(reports_page))
            .route("/data", web::get().to(reports_data))
            .route("/export", web::get().to(export_report))
            .route("/monitor/{id}", web::get().to(monitor_report))
    );
}

async fn reports_page(session: Session, db_pool: web::Data<SqlitePool>) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => {
            render_reports(&email, &db_pool).await
        }
        _ => {
            HttpResponse::Found()
                .append_header(("Location", "/"))
                .finish()
        }
    }
}

async fn render_reports(email: &str, db_pool: &SqlitePool) -> HttpResponse {
    let mut html = String::new();

    let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE email = ?", email)
        .fetch_optional(db_pool)
        .await
        .unwrap_or(None);

    if user_id.is_none() {
        return HttpResponse::Found()
            .append_header(("Location", "/"))
            .finish();
    }

    let user_id_val = user_id.unwrap();

    let monitors = sqlx::query!(
        "SELECT id, name, target_url FROM monitors WHERE user_id = ? ORDER BY name",
        user_id_val
    )
    .fetch_all(db_pool)
    .await
    .unwrap_or(vec![]);

    html.push_str(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Reports - Basic Monitoring</title>
    <link href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/sweetalert2@11"></script>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js"></script>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
        body { font-family: 'Inter', sans-serif; background-color: #f8fafc; }
        .sidebar-bg { background: linear-gradient(180deg, #1e293b 0%, #0f172a 100%); }
        .stat-card { transition: transform 0.2s ease; }
        .stat-card:hover { transform: translateY(-2px); }
        .loading-spinner { display: inline-block; width: 20px; height: 20px; border: 2px solid #e5e7eb; border-top-color: #3b82f6; border-radius: 50%; animation: spin 1s linear infinite; }
        @keyframes spin { to { transform: rotate(360deg); } }
        .date-range-preset { cursor: pointer; transition: all 0.2s ease; }
        .date-range-preset:hover { background-color: #f1f5f9; }
        .date-range-preset.active { background-color: #3b82f6; color: white; }
        .export-dropdown { position: relative; display: inline-block; }
        .export-dropdown-content { display: none; position: absolute; right: 0; background-color: white; min-width: 160px; box-shadow: 0px 8px 16px 0px rgba(0,0,0,0.2); z-index: 1; border-radius: 0.5rem; }
        .export-dropdown:hover .export-dropdown-content { display: block; }
        .export-dropdown-content a { color: black; padding: 12px 16px; text-decoration: none; display: block; border-radius: 0.5rem; }
        .export-dropdown-content a:hover { background-color: #f1f5f9; }
        .incident-row { transition: all 0.2s ease; }
        .incident-row:hover { background-color: #fef2f2; }
        .sla-card { background: linear-gradient(135deg, #1e293b 0%, #0f172a 100%); }
        .sla-met { background: linear-gradient(135deg, #10b981 0%, #059669 100%); }
        .sla-not-met { background: linear-gradient(135deg, #ef4444 0%, #dc2626 100%); }
        .uptime-chart-container { position: relative; height: 300px; width: 100%; }
    </style>
</head>
<body>"##);

    html.push_str(&navbar::render(email));
    html.push_str(r#"<div class="flex">"#);
    html.push_str(&sidebar::render());

    html.push_str(r##"
        <div class="flex-1 p-6">
            <div class="flex justify-between items-center mb-6">
                <div>
                    <h1 class="text-2xl font-bold text-gray-800">Performance Reports</h1>
                    <p class="text-gray-600">Analyze your website performance and SSL certificate status</p>
                </div>
                <div class="export-dropdown">
                    <button class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition flex items-center">
                        <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>
                        </svg>
                        Export Report
                    </button>
                    <div class="export-dropdown-content">
                        <a href="#" onclick="exportReport('pdf')">📄 Export as PDF</a>
                        <a href="#" onclick="exportReport('csv')">📊 Export as CSV</a>
                        <a href="#" onclick="exportReport('html')">🌐 Export as HTML</a>
                    </div>
                </div>
            </div>

            <!-- Filter Panel -->
            <div class="bg-white rounded-lg shadow p-6 mb-6">
                <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Monitor</label>
                        <select id="monitor-select" class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500">
                            <option value="all">All Monitors</option>"##);

    for monitor in &monitors {
        let id_value = monitor.id.unwrap_or(0);
        html.push_str(&format!(
            r#"<option value="{}">{}</option>"#,
            id_value, monitor.name
        ));
    }

    html.push_str(r##"
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Period</label>
                        <select id="period-select" class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500">
                            <option value="24h">Last 24 Hours</option>
                            <option value="7d" selected>Last 7 Days</option>
                            <option value="30d">Last 30 Days</option>
                            <option value="90d">Last 90 Days</option>
                            <option value="custom">Custom Range</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Start Date</label>
                        <input type="date" id="start-date" class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500" disabled>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">End Date</label>
                        <input type="date" id="end-date" class="w-full border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500" disabled>
                    </div>
                </div>

                <div class="flex flex-wrap gap-2 mt-4">
                    <span class="text-sm text-gray-600 mr-2 py-2">Quick Range:</span>
                    <button onclick="setPreset('today')" class="date-range-preset px-3 py-1 text-sm border border-gray-300 rounded-full hover:bg-gray-100">Today</button>
                    <button onclick="setPreset('yesterday')" class="date-range-preset px-3 py-1 text-sm border border-gray-300 rounded-full hover:bg-gray-100">Yesterday</button>
                    <button onclick="setPreset('7d')" class="date-range-preset px-3 py-1 text-sm border border-gray-300 rounded-full hover:bg-gray-100">Last 7 Days</button>
                    <button onclick="setPreset('30d')" class="date-range-preset px-3 py-1 text-sm border border-gray-300 rounded-full hover:bg-gray-100">Last 30 Days</button>
                    <button onclick="setPreset('thisMonth')" class="date-range-preset px-3 py-1 text-sm border border-gray-300 rounded-full hover:bg-gray-100">This Month</button>
                    <button onclick="setPreset('lastMonth')" class="date-range-preset px-3 py-1 text-sm border border-gray-300 rounded-full hover:bg-gray-100">Last Month</button>
                </div>

                <div class="flex justify-end mt-4">
                    <button onclick="loadReportData()" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition">
                        Generate Report
                    </button>
                </div>
            </div>

            <!-- SLA Report Card -->
            <div id="sla-card" class="rounded-lg shadow mb-6 hidden">
                <div class="p-6 text-white rounded-lg" id="sla-bg">
                    <div class="flex justify-between items-center">
                        <div>
                            <h3 class="text-lg font-semibold mb-2">📋 Service Level Agreement (SLA)</h3>
                            <p class="text-sm opacity-90">Target: <span id="sla-target">99.9</span>%</p>
                        </div>
                        <div class="text-right">
                            <p class="text-3xl font-bold" id="sla-achieved">0</p>
                            <p class="text-sm opacity-90">Achieved Uptime</p>
                        </div>
                        <div class="text-right">
                            <p class="text-2xl font-bold" id="sla-status">-</p>
                            <p class="text-sm opacity-90">Status</p>
                        </div>
                    </div>
                    <div class="mt-4 pt-4 border-t border-white/20">
                        <div class="grid grid-cols-3 gap-4 text-center">
                            <div>
                                <p class="text-2xl font-bold" id="total-downtime-display">0</p>
                                <p class="text-xs opacity-80">Total Downtime</p>
                            </div>
                            <div>
                                <p class="text-2xl font-bold" id="allowed-downtime">0</p>
                                <p class="text-xs opacity-80">Allowed Downtime</p>
                            </div>
                            <div>
                                <p class="text-2xl font-bold" id="sla-buffer">0</p>
                                <p class="text-xs opacity-80">Buffer</p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Stats Cards -->
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
                <div class="stat-card bg-white rounded-lg shadow p-4">
                    <div class="flex items-center">
                        <div class="w-10 h-10 bg-green-100 rounded-lg flex items-center justify-center mr-3">
                            <span class="text-green-600 text-xl">📈</span>
                        </div>
                        <div>
                            <p class="text-sm text-gray-600">Avg Uptime</p>
                            <p class="text-xl font-bold" id="avg-uptime">99.9%</p>
                        </div>
                    </div>
                </div>
                <div class="stat-card bg-white rounded-lg shadow p-4">
                    <div class="flex items-center">
                        <div class="w-10 h-10 bg-blue-100 rounded-lg flex items-center justify-center mr-3">
                            <span class="text-blue-600 text-xl">⏱️</span>
                        </div>
                        <div>
                            <p class="text-sm text-gray-600">Avg Response</p>
                            <p class="text-xl font-bold" id="avg-response">234ms</p>
                        </div>
                    </div>
                </div>
                <div class="stat-card bg-white rounded-lg shadow p-4">
                    <div class="flex items-center">
                        <div class="w-10 h-10 bg-yellow-100 rounded-lg flex items-center justify-center mr-3">
                            <span class="text-yellow-600 text-xl">🔔</span>
                        </div>
                        <div>
                            <p class="text-sm text-gray-600">Total Incidents</p>
                            <p class="text-xl font-bold" id="total-incidents">0</p>
                        </div>
                    </div>
                </div>
                <div class="stat-card bg-white rounded-lg shadow p-4">
                    <div class="flex items-center">
                        <div class="w-10 h-10 bg-purple-100 rounded-lg flex items-center justify-center mr-3">
                            <span class="text-purple-600 text-xl">⏰</span>
                        </div>
                        <div>
                            <p class="text-sm text-gray-600">Total Downtime</p>
                            <p class="text-xl font-bold" id="total-downtime">0m</p>
                        </div>
                    </div>
                </div>
            </div>

            <!-- SSL Certificate Stats Card -->
            <div class="bg-white rounded-lg shadow p-6 mb-6">
                <h3 class="text-lg font-semibold mb-4">🔒 SSL Certificate Summary</h3>
                <div class="grid grid-cols-2 md:grid-cols-5 gap-4">
                    <div class="text-center p-3 bg-slate-50 rounded-lg">
                        <div class="text-2xl font-bold text-slate-700" id="ssl-total">0</div>
                        <div class="text-xs text-slate-500">Total Certificates</div>
                    </div>
                    <div class="text-center p-3 bg-green-50 rounded-lg">
                        <div class="text-2xl font-bold text-green-600" id="ssl-valid">0</div>
                        <div class="text-xs text-green-600">Valid</div>
                    </div>
                    <div class="text-center p-3 bg-yellow-50 rounded-lg">
                        <div class="text-2xl font-bold text-yellow-600" id="ssl-expiring">0</div>
                        <div class="text-xs text-yellow-600">Expiring (≤7d)</div>
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
                <div class="mt-3 text-center">
                    <span class="text-sm text-gray-600">Average Days Left: </span>
                    <span id="ssl-avg-days" class="font-semibold">0</span> days
                </div>
            </div>

            <!-- Uptime Trend Chart -->
            <div class="bg-white rounded-lg shadow p-6 mb-6">
                <h3 class="text-lg font-semibold mb-4">📈 Uptime Trend (Last 30 Days)</h3>
                <div class="uptime-chart-container">
                    <canvas id="uptime-chart"></canvas>
                </div>
            </div>

            <!-- Monitor Performance Ranking -->
            <div class="bg-white rounded-lg shadow overflow-hidden mb-6">
                <div class="p-4 border-b border-gray-200">
                    <h3 class="text-lg font-semibold">Monitor Performance Ranking</h3>
                </div>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-gray-200">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Monitor</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Uptime</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Avg Response</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Total Checks</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Downtime</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Last Downtime</th>
                            </tr>
                        </thead>
                        <tbody id="monitor-ranking" class="bg-white divide-y divide-gray-200">
                            <tr>
                                <td colspan="6" class="px-6 py-12 text-center text-gray-500">
                                    <div class="inline-block loading-spinner mr-2"></div>
                                    Loading data...
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>

            <!-- Incident History -->
            <div class="bg-white rounded-lg shadow overflow-hidden mb-6">
                <div class="p-4 border-b border-gray-200">
                    <h3 class="text-lg font-semibold">🔴 Incident History (Downtime Events)</h3>
                    <p class="text-sm text-gray-500">Detailed record of all downtime incidents</p>
                </div>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-gray-200">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Date & Time</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Monitor</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Duration</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Severity</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Message</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Status</th>
                            </tr>
                        </thead>
                        <tbody id="incident-history" class="bg-white divide-y divide-gray-200">
                            <tr>
                                <td colspan="6" class="px-6 py-12 text-center text-gray-500">
                                    <div class="inline-block loading-spinner mr-2"></div>
                                    Loading incidents...
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>

            <!-- SSL Certificate List -->
            <div class="bg-white rounded-lg shadow overflow-hidden mb-6">
                <div class="p-4 border-b border-gray-200">
                    <h3 class="text-lg font-semibold">🔒 SSL Certificates</h3>
                    <p class="text-sm text-gray-500">Current SSL certificate status</p>
                </div>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-gray-200">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Domain</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Port</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Status</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Days Left</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Expiry Date</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Issuer</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Last Checked</th>
                            </tr>
                        </thead>
                        <tbody id="ssl-certificates" class="bg-white divide-y divide-gray-200">
                            <tr>
                                <td colspan="7" class="px-6 py-12 text-center text-gray-500">
                                    <div class="inline-block loading-spinner mr-2"></div>
                                    Loading SSL certificates...
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    </div>
    "##);

    html.push_str(&footer::render());

    html.push_str(r##"
    <script>
        let currentReportData = null;
        let uptimeChart = null;

        document.addEventListener('DOMContentLoaded', function() {
            console.log('Reports page loaded');
            setupEventListeners();
            loadReportData();
        });

        function setupEventListeners() {
            document.getElementById('period-select').addEventListener('change', function() {
                const isCustom = this.value === 'custom';
                document.getElementById('start-date').disabled = !isCustom;
                document.getElementById('end-date').disabled = !isCustom;

                if (!isCustom) {
                    loadReportData();
                }
            });

            document.getElementById('start-date').addEventListener('change', function() {
                if (document.getElementById('period-select').value === 'custom') {
                    loadReportData();
                }
            });

            document.getElementById('end-date').addEventListener('change', function() {
                if (document.getElementById('period-select').value === 'custom') {
                    loadReportData();
                }
            });

            document.getElementById('monitor-select').addEventListener('change', function() {
                loadReportData();
            });
        }

        function setPreset(preset) {
            const today = new Date();
            let startDate = new Date(today);
            let endDate = new Date(today);

            switch(preset) {
                case 'today':
                    startDate.setHours(0, 0, 0, 0);
                    endDate.setHours(23, 59, 59, 999);
                    break;
                case 'yesterday':
                    startDate.setDate(today.getDate() - 1);
                    startDate.setHours(0, 0, 0, 0);
                    endDate.setDate(today.getDate() - 1);
                    endDate.setHours(23, 59, 59, 999);
                    break;
                case '7d':
                    startDate.setDate(today.getDate() - 7);
                    startDate.setHours(0, 0, 0, 0);
                    endDate.setHours(23, 59, 59, 999);
                    break;
                case '30d':
                    startDate.setDate(today.getDate() - 30);
                    startDate.setHours(0, 0, 0, 0);
                    endDate.setHours(23, 59, 59, 999);
                    break;
                case 'thisMonth':
                    startDate = new Date(today.getFullYear(), today.getMonth(), 1);
                    endDate = new Date(today.getFullYear(), today.getMonth() + 1, 0);
                    endDate.setHours(23, 59, 59, 999);
                    break;
                case 'lastMonth':
                    startDate = new Date(today.getFullYear(), today.getMonth() - 1, 1);
                    endDate = new Date(today.getFullYear(), today.getMonth(), 0);
                    endDate.setHours(23, 59, 59, 999);
                    break;
                default:
                    startDate.setDate(today.getDate() - 7);
                    startDate.setHours(0, 0, 0, 0);
                    endDate.setHours(23, 59, 59, 999);
            }

            document.getElementById('start-date').value = startDate.toISOString().split('T')[0];
            document.getElementById('end-date').value = endDate.toISOString().split('T')[0];
            document.getElementById('period-select').value = 'custom';
            document.getElementById('start-date').disabled = false;
            document.getElementById('end-date').disabled = false;

            loadReportData();
        }

        function loadReportData() {
            const monitorId = document.getElementById('monitor-select').value;
            const period = document.getElementById('period-select').value;
            const startDate = document.getElementById('start-date').value;
            const endDate = document.getElementById('end-date').value;

            let url = `/reports/data?period=${period}`;
            if (monitorId !== 'all') url += `&monitor_id=${monitorId}`;
            if (period === 'custom' && startDate && endDate) {
                url += `&start_date=${startDate}&end_date=${endDate}`;
            }

            fetch(url)
                .then(response => response.json())
                .then(data => {
                    currentReportData = data;
                    updateStats(data.summary);
                    updateSlaCard(data.sla);
                    updateMonitorRanking(data.monitors);
                    updateIncidentHistory(data.incidents);
                    updateSslCertificates(data.ssl_certificates);
                    updateSslStats(data.ssl_summary);
                    updateUptimeChart(data.uptime_trend);
                })
                .catch(error => {
                    console.error('Error loading report:', error);
                    Swal.fire({
                        icon: 'error',
                        title: 'Error',
                        text: 'Failed to load report data'
                    });
                });
        }

        function updateStats(summary) {
            document.getElementById('avg-uptime').textContent = (summary.avg_uptime || 99.9).toFixed(2) + '%';
            document.getElementById('avg-response').textContent = Math.round(summary.avg_response || 0) + 'ms';
            document.getElementById('total-incidents').textContent = summary.total_incidents || 0;

            const totalDowntime = summary.total_downtime || 0;
            const hours = Math.floor(totalDowntime / 60);
            const minutes = totalDowntime % 60;
            document.getElementById('total-downtime').textContent =
                hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
            document.getElementById('total-downtime-display').textContent =
                hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
        }

        function updateSlaCard(sla) {
            if (!sla) return;

            const slaCard = document.getElementById('sla-card');
            const slaBg = document.getElementById('sla-bg');

            slaCard.classList.remove('hidden');

            const isMet = sla.status === 'MET';
            slaBg.className = 'p-6 rounded-lg ' + (isMet ? 'sla-met' : 'sla-not-met');

            document.getElementById('sla-target').textContent = sla.target_percent.toFixed(1);
            document.getElementById('sla-achieved').textContent = sla.achieved_percent.toFixed(2) + '%';
            document.getElementById('sla-status').textContent = sla.status;
            document.getElementById('allowed-downtime').textContent = formatDuration(sla.allowed_downtime_seconds);
            document.getElementById('sla-buffer').textContent = sla.buffer_seconds > 0 ? formatDuration(sla.buffer_seconds) : '0';
        }

        function formatDuration(seconds) {
            const hours = Math.floor(seconds / 3600);
            const minutes = Math.floor((seconds % 3600) / 60);
            if (hours > 0) return `${hours}h ${minutes}m`;
            if (minutes > 0) return `${minutes}m`;
            return `${seconds}s`;
        }

        function updateMonitorRanking(monitors) {
            const tbody = document.getElementById('monitor-ranking');
            if (!monitors || monitors.length === 0) {
                tbody.innerHTML = `
                    <tr>
                        <td colspan="6" class="px-6 py-8 text-center text-gray-500">
                            No data available for selected period
                        </td>
                    </tr>
                `;
                return;
            }

            let html = '';
            monitors.forEach(m => {
                const downtime = m.downtime || 0;
                const downtimeText = downtime > 0 ?
                    (downtime > 60 ? `${Math.floor(downtime/60)}h ${downtime%60}m` : `${downtime}m`) :
                    '0m';

                html += `
                    <tr class="hover:bg-gray-50">
                        <td class="px-6 py-4">
                            <div class="font-medium">${escapeHtml(m.name)}</div>
                            <div class="text-sm text-gray-500">${escapeHtml(m.url)}</div>
                        </td>
                        <td class="px-6 py-4">
                            <span class="px-2 py-1 ${m.uptime > 99.9 ? 'text-green-600' : 'text-yellow-600'} font-semibold">
                                ${(m.uptime || 0).toFixed(2)}%
                            </span>
                        </td>
                        <td class="px-6 py-4">${Math.round(m.avg_response || 0)}ms</td>
                        <td class="px-6 py-4">${m.total_checks || 0}</td>
                        <td class="px-6 py-4">${downtimeText}</td>
                        <td class="px-6 py-4 text-sm text-gray-500">${m.last_downtime || 'Never'}</td>
                    </tr>
                `;
            });
            tbody.innerHTML = html;
        }

        function updateIncidentHistory(incidents) {
            const tbody = document.getElementById('incident-history');
            if (!incidents || incidents.length === 0) {
                tbody.innerHTML = `
                    <tr>
                        <td colspan="6" class="px-6 py-8 text-center text-gray-500">
                            No incidents reported in selected period
                        </td>
                    </tr>
                `;
                return;
            }

            let html = '';
            incidents.forEach(incident => {
                const severityClass = {
                    'critical': 'bg-red-100 text-red-800',
                    'warning': 'bg-yellow-100 text-yellow-800',
                    'info': 'bg-blue-100 text-blue-800'
                }[incident.severity] || 'bg-gray-100 text-gray-800';

                const statusBadge = incident.status === 'resolved'
                    ? '<span class="px-2 py-1 text-xs rounded-full bg-green-100 text-green-800">Resolved</span>'
                    : '<span class="px-2 py-1 text-xs rounded-full bg-yellow-100 text-yellow-800">Active</span>';

                html += `
                    <tr class="incident-row hover:bg-red-50">
                        <td class="px-6 py-4 text-sm whitespace-nowrap">
                            ${new Date(incident.triggered_at).toLocaleString()}
                        </td>
                        <td class="px-6 py-4">
                            <div class="font-medium">${escapeHtml(incident.monitor_name)}</div>
                            <div class="text-xs text-gray-500">${escapeHtml(incident.monitor_url)}</div>
                        </td>
                        <td class="px-6 py-4 whitespace-nowrap">
                            <span class="font-mono font-medium">${incident.duration_display}</span>
                        </td>
                        <td class="px-6 py-4">
                            <span class="px-2 py-1 text-xs font-semibold rounded-full ${severityClass}">
                                ${incident.severity}
                            </span>
                        </td>
                        <td class="px-6 py-4">
                            <div class="max-w-xs">
                                <p class="text-sm text-gray-700">${escapeHtml(incident.error_message)}</p>
                            </div>
                        </td>
                        <td class="px-6 py-4">${statusBadge}</td>
                    </tr>
                `;
            });
            tbody.innerHTML = html;
        }

        function updateSslCertificates(certificates) {
            const tbody = document.getElementById('ssl-certificates');
            if (!certificates || certificates.length === 0) {
                tbody.innerHTML = `
                    <tr>
                        <td colspan="7" class="px-6 py-8 text-center text-gray-500">
                            No SSL certificates found
                        </td>
                    </tr>
                `;
                return;
            }

            let html = '';
            certificates.forEach(cert => {
                let statusClass = '';
                let statusText = '';
                switch(cert.status) {
                    case 'valid':
                        statusClass = 'bg-green-100 text-green-800';
                        statusText = '✅ Valid';
                        break;
                    case 'expiring':
                        statusClass = 'bg-yellow-100 text-yellow-800';
                        statusText = '⚠️ Expiring Soon';
                        break;
                    case 'expired':
                        statusClass = 'bg-red-100 text-red-800';
                        statusText = '❌ Expired';
                        break;
                    default:
                        statusClass = 'bg-gray-100 text-gray-800';
                        statusText = '⚪ Unknown';
                }

                const daysDisplay = cert.days_left < 0 ? 'Expired' : `${cert.days_left} days`;
                const expiryDisplay = cert.expiry_date || 'Not checked';
                const issuerDisplay = cert.issuer || 'Unknown';
                const lastCheckedDisplay = cert.last_checked ? new Date(cert.last_checked).toLocaleString() : 'Never';

                html += `
                    <tr class="hover:bg-gray-50">
                        <td class="px-6 py-4 font-medium">${escapeHtml(cert.domain)}</td>
                        <td class="px-6 py-4">${cert.port}</td>
                        <td class="px-6 py-4">
                            <span class="px-2 py-1 text-xs font-semibold rounded-full ${statusClass}">
                                ${statusText}
                            </span>
                        </td>
                        <td class="px-6 py-4">${daysDisplay}</td>
                        <td class="px-6 py-4 text-sm">${expiryDisplay}</td>
                        <td class="px-6 py-4 text-sm">${escapeHtml(issuerDisplay)}</td>
                        <td class="px-6 py-4 text-sm">${lastCheckedDisplay}</td>
                    </tr>
                `;
            });
            tbody.innerHTML = html;
        }

        function updateSslStats(sslSummary) {
            if (sslSummary) {
                document.getElementById('ssl-total').textContent = sslSummary.total_certificates || 0;
                document.getElementById('ssl-valid').textContent = sslSummary.valid || 0;
                document.getElementById('ssl-expiring').textContent = sslSummary.expiring || 0;
                document.getElementById('ssl-expired').textContent = sslSummary.expired || 0;
                document.getElementById('ssl-unknown').textContent = sslSummary.unknown || 0;
                document.getElementById('ssl-avg-days').textContent = (sslSummary.avg_days_left || 0).toFixed(1);
            }
        }

        function updateUptimeChart(trend) {
            if (!trend || !trend.labels || trend.labels.length === 0) return;

            const ctx = document.getElementById('uptime-chart').getContext('2d');

            if (uptimeChart) {
                uptimeChart.destroy();
            }

            uptimeChart = new Chart(ctx, {
                type: 'line',
                data: {
                    labels: trend.labels,
                    datasets: [{
                        label: 'Uptime (%)',
                        data: trend.values,
                        borderColor: '#10b981',
                        backgroundColor: 'rgba(16, 185, 129, 0.1)',
                        fill: true,
                        tension: 0.4,
                        pointRadius: 3,
                        pointBackgroundColor: '#10b981',
                        pointBorderColor: '#fff',
                        pointBorderWidth: 2
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        tooltip: {
                            callbacks: {
                                label: function(context) {
                                    return `Uptime: ${context.raw}%`;
                                }
                            }
                        },
                        legend: {
                            position: 'top'
                        }
                    },
                    scales: {
                        y: {
                            min: 95,
                            max: 100,
                            ticks: {
                                callback: function(value) {
                                    return value + '%';
                                }
                            },
                            title: {
                                display: true,
                                text: 'Uptime Percentage (%)'
                            }
                        },
                        x: {
                            title: {
                                display: true,
                                text: 'Date'
                            }
                        }
                    }
                }
            });
        }

        function escapeHtml(text) {
            if (!text) return '';
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        function exportReport(format) {
            if (!currentReportData) {
                Swal.fire({
                    icon: 'warning',
                    title: 'No Data',
                    text: 'Please generate a report first'
                });
                return;
            }

            const monitorId = document.getElementById('monitor-select').value;
            const period = document.getElementById('period-select').value;
            const startDate = document.getElementById('start-date').value;
            const endDate = document.getElementById('end-date').value;

            let url = `/reports/export?format=${format}&period=${period}`;
            if (monitorId !== 'all') url += `&monitor_id=${monitorId}`;
            if (period === 'custom' && startDate && endDate) {
                url += `&start_date=${startDate}&end_date=${endDate}`;
            }

            window.open(url, '_blank');
        }
    </script>
    </body>
    </html>
    "##);

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

// ==================== API ENDPOINTS ====================

#[derive(Serialize)]
struct ReportSummary {
    avg_uptime: f64,
    avg_response: f64,
    total_alerts: i64,
    total_downtime: i64,
    total_incidents: i64,
}

#[derive(Serialize)]
struct SlaReport {
    target_percent: f64,
    achieved_percent: f64,
    status: String,
    total_downtime_seconds: i64,
    allowed_downtime_seconds: i64,
    buffer_seconds: i64,
}

#[derive(Serialize)]
struct UptimeTrend {
    labels: Vec<String>,
    values: Vec<f64>,
}

#[derive(Serialize)]
struct IncidentReport {
    id: i64,
    monitor_name: String,
    monitor_url: String,
    triggered_at: String,
    resolved_at: String,
    duration_seconds: i64,
    duration_display: String,
    severity: String,
    error_message: String,
    status: String,
}

#[derive(Serialize)]
struct SslReportSummary {
    total_certificates: i64,
    valid: i64,
    expiring: i64,
    expired: i64,
    unknown: i64,
    avg_days_left: f64,
}

#[derive(Serialize)]
struct MonitorReport {
    name: String,
    url: String,
    uptime: f64,
    avg_response: f64,
    total_checks: i64,
    downtime: i64,
    last_downtime: Option<String>,
}

#[derive(Serialize)]
struct SslCertificateReport {
    domain: String,
    port: i64,
    expiry_date: Option<String>,
    days_left: i64,
    status: String,
    issuer: String,
    last_checked: Option<String>,
}

#[derive(Serialize)]
struct ChartData {
    labels: Vec<String>,
    response_times: Vec<f64>,
    uptime_stats: UptimeStats,
}

#[derive(Serialize)]
struct UptimeStats {
    up: i64,
    down: i64,
    slow: i64,
}

// ==================== HELPER FUNCTION ====================

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

fn format_duration_seconds(seconds: i64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

fn calculate_sla(uptime: f64, period_days: i64) -> SlaReport {
    let target = 99.9;
    let achieved = uptime;
    let total_seconds = period_days * 86400;
    let allowed_downtime = (100.0 - target) / 100.0 * total_seconds as f64;
    let actual_downtime = (100.0 - achieved) / 100.0 * total_seconds as f64;

    SlaReport {
        target_percent: target,
        achieved_percent: achieved,
        status: if achieved >= target { "MET".to_string() } else { "NOT MET".to_string() },
        total_downtime_seconds: actual_downtime as i64,
        allowed_downtime_seconds: allowed_downtime as i64,
        buffer_seconds: (allowed_downtime - actual_downtime) as i64,
    }
}

// ==================== DATA HANDLERS ====================

async fn reports_data(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    query: web::Query<ReportQuery>,
) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let (start_date, end_date) = parse_date_range(&query).await;
                    let period_days = calculate_period_days(&start_date, &end_date).await;

                    // Website monitors data
                    let monitors = sqlx::query_as::<_, (i64, String, String)>(
                        "SELECT id, name, target_url FROM monitors WHERE user_id = ?"
                    )
                    .bind(user_id)
                    .fetch_all(db_pool.get_ref())
                    .await
                    .unwrap_or(vec![]);

                    let mut monitor_reports = Vec::new();
                    let mut total_uptime = 0.0;
                    let mut total_response = 0.0;
                    let mut total_downtime = 0;
                    let mut total_incidents = 0;
                    let mut monitor_count = 0;

                    for (id, name, url) in monitors {
                        if let Some(monitor_id) = query.monitor_id {
                            if id != monitor_id {
                                continue;
                            }
                        }

                        monitor_count += 1;

                        let uptime_stats = sqlx::query(
                            "SELECT
                                COUNT(*) as total,
                                SUM(CASE WHEN status = 'up' THEN 1 ELSE 0 END) as up_count,
                                COALESCE(AVG(response_time), 0) as avg_resp
                             FROM check_results
                             WHERE monitor_id = ? AND checked_at BETWEEN ? AND ?"
                        )
                        .bind(id)
                        .bind(&start_date)
                        .bind(&end_date)
                        .fetch_one(db_pool.get_ref())
                        .await;

                        let (total, up_count, avg_resp) = match uptime_stats {
                            Ok(row) => {
                                let total_val: i64 = row.get(0);
                                let up_val: Option<i64> = row.get(1);
                                let avg_val: f64 = row.get(2);
                                (total_val, up_val.unwrap_or(0), avg_val)
                            },
                            Err(_) => (0, 0, 0.0)
                        };

                        let uptime = if total > 0 {
                            (up_count as f64 / total as f64) * 100.0
                        } else {
                            100.0
                        };

                        let downtime = sqlx::query_scalar::<_, i64>(
                            "SELECT COALESCE(SUM(
                                strftime('%s', resolved_at) - strftime('%s', triggered_at)
                            ) / 60, 0) FROM alerts
                             WHERE monitor_id = ? AND triggered_at BETWEEN ? AND ? AND status = 'resolved'"
                        )
                        .bind(id)
                        .bind(&start_date)
                        .bind(&end_date)
                        .fetch_one(db_pool.get_ref())
                        .await
                        .unwrap_or(0);

                        let incident_count = sqlx::query_scalar::<_, i64>(
                            "SELECT COUNT(*) FROM alerts
                             WHERE monitor_id = ? AND triggered_at BETWEEN ? AND ? AND severity = 'critical'"
                        )
                        .bind(id)
                        .bind(&start_date)
                        .bind(&end_date)
                        .fetch_one(db_pool.get_ref())
                        .await
                        .unwrap_or(0);

                        let last_downtime = sqlx::query_scalar::<_, String>(
                            "SELECT triggered_at FROM alerts
                             WHERE monitor_id = ? AND status = 'resolved'
                             ORDER BY triggered_at DESC LIMIT 1"
                        )
                        .bind(id)
                        .fetch_optional(db_pool.get_ref())
                        .await
                        .unwrap_or(None);

                        monitor_reports.push(MonitorReport {
                            name,
                            url,
                            uptime,
                            avg_response: avg_resp,
                            total_checks: total,
                            downtime,
                            last_downtime,
                        });

                        total_uptime += uptime;
                        total_response += avg_resp;
                        total_downtime += downtime;
                        total_incidents += incident_count;
                    }

                    let avg_uptime = if monitor_count > 0 { total_uptime / monitor_count as f64 } else { 100.0 };
                    let avg_response = if monitor_count > 0 { total_response / monitor_count as f64 } else { 0.0 };

                    let summary = ReportSummary {
                        avg_uptime,
                        avg_response,
                        total_alerts: total_incidents,
                        total_downtime,
                        total_incidents,
                    };

                    let sla = calculate_sla(avg_uptime, period_days);

                    // Uptime trend data
                    let uptime_trend = get_uptime_trend(db_pool.get_ref(), user_id, &start_date, &end_date).await;

                    // Incident history
                    let incidents = get_incident_history(db_pool.get_ref(), user_id, query.monitor_id, &start_date, &end_date).await;

                    // SSL certificates data
                    let ssl_monitors = sqlx::query!(
                        "SELECT id, domain, port, expiry_date, issuer, last_checked, last_status
                         FROM ssl_monitors
                         WHERE user_id = ?",
                        user_id
                    )
                    .fetch_all(db_pool.get_ref())
                    .await
                    .unwrap_or(vec![]);

                    let mut ssl_reports = Vec::new();
                    let mut ssl_valid = 0;
                    let mut ssl_expiring = 0;
                    let mut ssl_expired = 0;
                    let mut ssl_unknown = 0;
                    let mut total_days_left = 0.0;
                    let mut days_count = 0;

                    for monitor in &ssl_monitors {
                        let expiry_date = monitor.expiry_date;
                        let (status, days_left) = calculate_ssl_status(&expiry_date);

                        match status.as_str() {
                            "valid" => ssl_valid += 1,
                            "expiring" => ssl_expiring += 1,
                            "expired" => ssl_expired += 1,
                            _ => ssl_unknown += 1,
                        }

                        if days_left > 0 {
                            total_days_left += days_left as f64;
                            days_count += 1;
                        }

                        ssl_reports.push(SslCertificateReport {
                            domain: monitor.domain.clone(),
                            port: monitor.port.unwrap_or(443),
                            expiry_date: expiry_date.map(|d| d.format("%Y-%m-%d").to_string()),
                            days_left,
                            status,
                            issuer: monitor.issuer.clone().unwrap_or_else(|| "Unknown".to_string()),
                            last_checked: monitor.last_checked.map(|d| d.to_string()),
                        });
                    }

                    let ssl_summary = SslReportSummary {
                        total_certificates: ssl_monitors.len() as i64,
                        valid: ssl_valid,
                        expiring: ssl_expiring,
                        expired: ssl_expired,
                        unknown: ssl_unknown,
                        avg_days_left: if days_count > 0 { total_days_left / days_count as f64 } else { 0.0 },
                    };

                    HttpResponse::Ok().json(serde_json::json!({
                        "summary": summary,
                        "sla": sla,
                        "monitors": monitor_reports,
                        "incidents": incidents,
                        "uptime_trend": uptime_trend,
                        "ssl_summary": ssl_summary,
                        "ssl_certificates": ssl_reports,
                    }))
                }
                _ => HttpResponse::Unauthorized().json(serde_json::json!({
                    "status": "error",
                    "message": "User not found"
                }))
            }
        }
        _ => HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "Not authenticated"
        })),
    }
}

async fn parse_date_range(query: &web::Query<ReportQuery>) -> (String, String) {
    let now = Utc::now();
    let (start, end) = match query.period.as_deref() {
        Some("24h") => (now - Duration::hours(24), now),
        Some("7d") => (now - Duration::days(7), now),
        Some("30d") => (now - Duration::days(30), now),
        Some("90d") => (now - Duration::days(90), now),
        Some("custom") => {
            let start = query.start_date.clone().unwrap_or_else(|| {
                (now - Duration::days(7)).format("%Y-%m-%d").to_string()
            });
            let end = query.end_date.clone().unwrap_or_else(|| {
                now.format("%Y-%m-%d").to_string()
            });
            return (format!("{} 00:00:00", start), format!("{} 23:59:59", end));
        }
        _ => (now - Duration::days(7), now),
    };

    (start.format("%Y-%m-%d 00:00:00").to_string(),
     end.format("%Y-%m-%d 23:59:59").to_string())
}

async fn calculate_period_days(start_date: &str, end_date: &str) -> i64 {
    if let (Ok(start), Ok(end)) = (
        NaiveDateTime::parse_from_str(start_date, "%Y-%m-%d %H:%M:%S"),
        NaiveDateTime::parse_from_str(end_date, "%Y-%m-%d %H:%M:%S")
    ) {
        (end - start).num_days().max(1)
    } else {
        7
    }
}

async fn get_uptime_trend(db_pool: &SqlitePool, user_id: i64, start: &str, end: &str) -> UptimeTrend {
    let daily_stats = sqlx::query(
        "SELECT
            date(cr.checked_at) as day,
            COUNT(CASE WHEN cr.status = 'up' THEN 1 END) as up_count,
            COUNT(*) as total_count
         FROM check_results cr
         JOIN monitors m ON cr.monitor_id = m.id
         WHERE m.user_id = ? AND cr.checked_at BETWEEN ? AND ?
         GROUP BY date(cr.checked_at)
         ORDER BY day
         LIMIT 30"
    )
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_all(db_pool)
    .await
    .unwrap_or(vec![]);

    let mut labels = Vec::new();
    let mut values = Vec::new();

    for row in daily_stats {
        if let Ok(day) = row.try_get::<String, _>("day") {
            labels.push(day);
        }
        let up_count: i64 = row.get("up_count");
        let total_count: i64 = row.get("total_count");
        let uptime = if total_count > 0 {
            (up_count as f64 / total_count as f64) * 100.0
        } else {
            100.0
        };
        values.push(uptime);
    }

    UptimeTrend { labels, values }
}

async fn get_incident_history(
    db_pool: &SqlitePool,
    user_id: i64,
    monitor_filter: Option<i64>,
    start_date: &str,
    end_date: &str,
) -> Vec<IncidentReport> {
    let mut conditions = vec![
        "a.user_id = ?".to_string(),
        "a.triggered_at BETWEEN ? AND ?".to_string(),
        "a.status = 'resolved'".to_string(),
    ];

    if let Some(monitor_id) = monitor_filter {
        conditions.push(format!("a.monitor_id = {}", monitor_id));
    }

    let where_clause = conditions.join(" AND ");

    let query = format!(
        "SELECT a.id, a.title, a.message, a.severity, a.triggered_at, a.resolved_at, a.status,
                m.name as monitor_name, m.target_url as monitor_url,
                (strftime('%s', a.resolved_at) - strftime('%s', a.triggered_at)) as duration_seconds
         FROM alerts a
         JOIN monitors m ON a.monitor_id = m.id
         WHERE {}
         ORDER BY a.triggered_at DESC
         LIMIT 50",
        where_clause
    );

    let rows = sqlx::query(&query)
        .bind(user_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(db_pool)
        .await
        .unwrap_or(vec![]);

    let mut incidents = Vec::new();

    for row in rows {
        let duration_secs: i64 = row.get("duration_seconds");

        incidents.push(IncidentReport {
            id: row.get("id"),
            monitor_name: row.get("monitor_name"),
            monitor_url: row.get("monitor_url"),
            triggered_at: row.get("triggered_at"),
            resolved_at: row.get("resolved_at"),
            duration_seconds: duration_secs,
            duration_display: format_duration_seconds(duration_secs),
            severity: row.get("severity"),
            error_message: row.get("message"),
            status: row.get("status"),
        });
    }

    incidents
}

// ==================== EXPORT FUNCTIONS ====================

async fn export_report(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    query: web::Query<ReportQuery>,
) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(db_pool.get_ref())
                .await
            {
                Ok(Some(user_id)) => {
                    let (start_date, end_date) = parse_date_range(&query).await;
                    let format = query.format.as_deref().unwrap_or("csv");
                    let period_days = calculate_period_days(&start_date, &end_date).await;

                    // Get all data for export
                    let monitors = get_export_monitors(db_pool.get_ref(), user_id, query.monitor_id, &start_date, &end_date).await;
                    let ssl_certificates = get_export_ssl_certificates(db_pool.get_ref(), user_id).await;
                    let incidents = get_incident_history(db_pool.get_ref(), user_id, query.monitor_id, &start_date, &end_date).await;
                    let uptime_trend = get_uptime_trend(db_pool.get_ref(), user_id, &start_date, &end_date).await;

                    let total_uptime: f64 = monitors.iter().map(|m| m.uptime).sum();
                    let avg_uptime = if monitors.len() > 0 { total_uptime / monitors.len() as f64 } else { 100.0 };
                    let sla = calculate_sla(avg_uptime, period_days);

                    match format {
                        "csv" => export_csv_report(monitors, ssl_certificates, incidents, sla, uptime_trend, &start_date, &end_date),
                        "json" => export_json_report(monitors, ssl_certificates, incidents, sla, uptime_trend, &start_date, &end_date),
                        "html" => export_html_report(monitors, ssl_certificates, incidents, sla, uptime_trend, &start_date, &end_date),
                        "pdf" => export_html_report(monitors, ssl_certificates, incidents, sla, uptime_trend, &start_date, &end_date),
                        _ => export_csv_report(monitors, ssl_certificates, incidents, sla, uptime_trend, &start_date, &end_date),
                    }
                }
                _ => HttpResponse::Unauthorized().body("Unauthorized"),
            }
        }
        _ => HttpResponse::Unauthorized().body("Unauthorized"),
    }
}

async fn get_export_monitors(
    db_pool: &SqlitePool,
    user_id: i64,
    monitor_filter: Option<i64>,
    start_date: &str,
    end_date: &str,
) -> Vec<MonitorReport> {
    let monitors = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, target_url FROM monitors WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_all(db_pool)
    .await
    .unwrap_or(vec![]);

    let mut monitor_reports = Vec::new();

    for (id, name, url) in monitors {
        if let Some(monitor_id) = monitor_filter {
            if id != monitor_id {
                continue;
            }
        }

        let uptime_stats = sqlx::query(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'up' THEN 1 ELSE 0 END) as up_count,
                COALESCE(AVG(response_time), 0) as avg_resp
             FROM check_results
             WHERE monitor_id = ? AND checked_at BETWEEN ? AND ?"
        )
        .bind(id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(db_pool)
        .await;

        let (total, up_count, avg_resp) = match uptime_stats {
            Ok(row) => {
                let total_val: i64 = row.get(0);
                let up_val: Option<i64> = row.get(1);
                let avg_val: f64 = row.get(2);
                (total_val, up_val.unwrap_or(0), avg_val)
            },
            Err(_) => (0, 0, 0.0)
        };

        let uptime = if total > 0 {
            (up_count as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        let downtime = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(SUM(
                strftime('%s', resolved_at) - strftime('%s', triggered_at)
            ) / 60, 0) FROM alerts
             WHERE monitor_id = ? AND triggered_at BETWEEN ? AND ? AND status = 'resolved'"
        )
        .bind(id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(db_pool)
        .await
        .unwrap_or(0);

        let last_downtime = sqlx::query_scalar::<_, String>(
            "SELECT triggered_at FROM alerts
             WHERE monitor_id = ? AND status = 'resolved'
             ORDER BY triggered_at DESC LIMIT 1"
        )
        .bind(id)
        .fetch_optional(db_pool)
        .await
        .unwrap_or(None);

        monitor_reports.push(MonitorReport {
            name,
            url,
            uptime,
            avg_response: avg_resp,
            total_checks: total,
            downtime,
            last_downtime,
        });
    }

    monitor_reports
}

async fn get_export_ssl_certificates(
    db_pool: &SqlitePool,
    user_id: i64,
) -> Vec<SslCertificateReport> {
    let ssl_monitors = sqlx::query!(
        "SELECT domain, port, expiry_date, issuer, last_checked, last_status
         FROM ssl_monitors
         WHERE user_id = ?",
        user_id
    )
    .fetch_all(db_pool)
    .await
    .unwrap_or(vec![]);

    let mut ssl_reports = Vec::new();

    for monitor in &ssl_monitors {
        let expiry_date = monitor.expiry_date;
        let (status, days_left) = calculate_ssl_status(&expiry_date);

        ssl_reports.push(SslCertificateReport {
            domain: monitor.domain.clone(),
            port: monitor.port.unwrap_or(443),
            expiry_date: expiry_date.map(|d| d.format("%Y-%m-%d").to_string()),
            days_left,
            status,
            issuer: monitor.issuer.clone().unwrap_or_else(|| "Unknown".to_string()),
            last_checked: monitor.last_checked.map(|d| d.to_string()),
        });
    }

    ssl_reports
}

fn export_csv_report(
    monitors: Vec<MonitorReport>,
    ssl_certificates: Vec<SslCertificateReport>,
    incidents: Vec<IncidentReport>,
    sla: SlaReport,
    _uptime_trend: UptimeTrend,
    start_date: &str,
    end_date: &str,
) -> HttpResponse {
    let mut csv_data = String::new();

    csv_data.push_str("=== MONITORING REPORT ===\n\n");
    csv_data.push_str(&format!("Period: {} to {}\n", start_date, end_date));
    csv_data.push_str(&format!("Generated: {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S")));

    csv_data.push_str("SLA REPORT\n");
    csv_data.push_str(&format!("Target SLA,{:.1}%\n", sla.target_percent));
    csv_data.push_str(&format!("Achieved Uptime,{:.2}%\n", sla.achieved_percent));
    csv_data.push_str(&format!("Status,{}\n", sla.status));
    csv_data.push_str(&format!("Total Downtime,{}\n", format_duration_seconds(sla.total_downtime_seconds)));
    csv_data.push_str(&format!("Allowed Downtime,{}\n", format_duration_seconds(sla.allowed_downtime_seconds)));
    csv_data.push_str(&format!("Buffer,{}\n\n", format_duration_seconds(sla.buffer_seconds)));

    csv_data.push_str("MONITOR PERFORMANCE\n");
    csv_data.push_str("Name,URL,Uptime (%),Avg Response (ms),Total Checks,Downtime (min),Last Downtime\n");
    for m in monitors {
        csv_data.push_str(&format!(
            "{},{},{:.2},{:.0},{},{},{}\n",
            escape_csv(&m.name),
            escape_csv(&m.url),
            m.uptime,
            m.avg_response,
            m.total_checks,
            m.downtime,
            m.last_downtime.as_deref().unwrap_or("Never")
        ));
    }
    csv_data.push('\n');

    csv_data.push_str("SSL CERTIFICATES\n");
    csv_data.push_str("Domain,Port,Status,Days Left,Expiry Date,Issuer,Last Checked\n");
    for cert in ssl_certificates {
        let days_display = if cert.days_left < 0 { "Expired" } else { &format!("{}", cert.days_left) };
        csv_data.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            escape_csv(&cert.domain),
            cert.port,
            cert.status,
            days_display,
            cert.expiry_date.as_deref().unwrap_or("Not checked"),
            escape_csv(&cert.issuer),
            cert.last_checked.as_deref().unwrap_or("Never")
        ));
    }
    csv_data.push('\n');

    csv_data.push_str("INCIDENT HISTORY\n");
    csv_data.push_str("Date & Time,Monitor,Duration,Severity,Message,Status\n");
    for inc in incidents {
        csv_data.push_str(&format!(
            "{},{},{},{},{},{}\n",
            inc.triggered_at,
            escape_csv(&inc.monitor_name),
            inc.duration_display,
            inc.severity,
            escape_csv(&inc.error_message),
            inc.status
        ));
    }

    HttpResponse::Ok()
        .content_type("text/csv; charset=utf-8")
        .append_header(("Content-Disposition", "attachment; filename=\"monitoring_report.csv\""))
        .body(csv_data)
}

fn export_json_report(
    monitors: Vec<MonitorReport>,
    ssl_certificates: Vec<SslCertificateReport>,
    incidents: Vec<IncidentReport>,
    sla: SlaReport,
    uptime_trend: UptimeTrend,
    start_date: &str,
    end_date: &str,
) -> HttpResponse {
    let json_data = serde_json::json!({
        "generated_at": Utc::now().to_rfc3339(),
        "period": {
            "start": start_date,
            "end": end_date
        },
        "sla": sla,
        "monitors": monitors,
        "incidents": incidents,
        "uptime_trend": uptime_trend,
        "ssl_certificates": ssl_certificates,
    });

    HttpResponse::Ok()
        .content_type("application/json; charset=utf-8")
        .append_header(("Content-Disposition", "attachment; filename=\"monitoring_report.json\""))
        .body(serde_json::to_string_pretty(&json_data).unwrap())
}

fn export_html_report(
    monitors: Vec<MonitorReport>,
    ssl_certificates: Vec<SslCertificateReport>,
    incidents: Vec<IncidentReport>,
    sla: SlaReport,
    _uptime_trend: UptimeTrend,
    start_date: &str,
    end_date: &str,
) -> HttpResponse {
    let _ssl_valid = ssl_certificates.iter().filter(|c| c.status == "valid").count();
    let _ssl_expiring = ssl_certificates.iter().filter(|c| c.status == "expiring").count();
    let _ssl_expired = ssl_certificates.iter().filter(|c| c.status == "expired").count();
    let _ssl_unknown = ssl_certificates.iter().filter(|c| c.status == "unknown").count();
    let total_downtime_seconds = sla.total_downtime_seconds;

    let avg_uptime = if monitors.is_empty() { 100.0 } else { monitors.iter().map(|m| m.uptime).sum::<f64>() / monitors.len() as f64 };
    let avg_response = if monitors.is_empty() { 0.0 } else { monitors.iter().map(|m| m.avg_response).sum::<f64>() / monitors.len() as f64 };
    let total_incidents = incidents.len();

    let sla_bg_start = if sla.status == "MET" { "#10b981" } else { "#ef4444" };
    let sla_bg_end = if sla.status == "MET" { "#059669" } else { "#dc2626" };

    let monitor_rows: String = monitors.iter().map(|m| {
        format!("<tr><td>{}</td><td>{}</td><td>{:.2}%</td><td>{:.0}ms</td><td>{}</td><td>{}m</td></tr>",
            escape_html(&m.name), escape_html(&m.url), m.uptime, m.avg_response, m.total_checks, m.downtime)
    }).collect();

    let incident_rows: String = incidents.iter().map(|i| {
        format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            i.triggered_at, escape_html(&i.monitor_name), i.duration_display, i.severity, escape_html(&i.error_message), i.status)
    }).collect();

    let ssl_rows: String = ssl_certificates.iter().map(|c| {
        let status_class = match c.status.as_str() {
            "valid" => "status-valid",
            "expiring" => "status-expiring",
            "expired" => "status-expired",
            _ => "",
        };
        let status_text = match c.status.as_str() {
            "valid" => "✅ Valid",
            "expiring" => "⚠️ Expiring Soon",
            "expired" => "❌ Expired",
            _ => "⚪ Unknown",
        };
        let days_display = if c.days_left < 0 {
            "Expired".to_string()
        } else {
            format!("{} days", c.days_left)
        };
        format!("<tr><td>{}</td><td class=\"{}\">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&c.domain), status_class, status_text, days_display,
            c.expiry_date.as_deref().unwrap_or("-"), escape_html(&c.issuer))
    }).collect();

    let total_downtime_display = format_duration_seconds(total_downtime_seconds);
    let allowed_downtime_display = format_duration_seconds(sla.allowed_downtime_seconds);
    let buffer_display = format_duration_seconds(sla.buffer_seconds);

    let html = format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Monitoring Report</title>
    <style>
        body {{ font-family: 'Segoe UI', Arial, sans-serif; margin: 40px; background: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; border-bottom: 3px solid #3b82f6; padding-bottom: 10px; }}
        h2 {{ color: #555; margin: 25px 0 15px; padding-left: 10px; border-left: 4px solid #3b82f6; }}
        .sla-card {{
            background: linear-gradient(135deg, {} 0%, {} 100%);
            color: white;
            padding: 20px;
            border-radius: 10px;
            margin: 20px 0;
        }}
        .summary-cards {{ display: flex; gap: 20px; margin: 20px 0; flex-wrap: wrap; }}
        .card {{ background: #f8fafc; padding: 20px; border-radius: 8px; flex: 1; min-width: 150px; text-align: center; }}
        .card-value {{ font-size: 28px; font-weight: bold; color: #3b82f6; }}
        table {{ width: 100%; border-collapse: collapse; margin: 15px 0; font-size: 13px; }}
        th {{ background: #3b82f6; color: white; padding: 10px; text-align: left; }}
        td {{ padding: 8px; border-bottom: 1px solid #e5e7eb; }}
        tr:hover {{ background: #f9fafb; }}
        .status-valid {{ color: #10b981; font-weight: bold; }}
        .status-expiring {{ color: #f59e0b; font-weight: bold; }}
        .status-expired {{ color: #ef4444; font-weight: bold; }}
        .footer {{ margin-top: 30px; padding-top: 20px; border-top: 1px solid #ddd; text-align: center; font-size: 11px; color: #666; }}
        @media print {{
            body {{ background: white; padding: 0; }}
            .container {{ box-shadow: none; }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>📊 Monitoring Report</h1>
        <p>Period: {} to {}</p>
        <p>Generated: {}</p>

        <div class="sla-card">
            <h3 style="margin: 0 0 10px;">📋 Service Level Agreement (SLA)</h3>
            <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap;">
                <div><strong>Target:</strong> {:.1}%</div>
                <div><strong>Achieved:</strong> {:.2}%</div>
                <div><strong>Status:</strong> {}</div>
                <div><strong>Total Downtime:</strong> {}</div>
                <div><strong>Allowed Downtime:</strong> {}</div>
                <div><strong>Buffer:</strong> {}</div>
            </div>
        </div>

        <div class="summary-cards">
            <div class="card"><div class="card-value">{:.1}%</div><div>Avg Uptime</div></div>
            <div class="card"><div class="card-value">{:.0}ms</div><div>Avg Response</div></div>
            <div class="card"><div class="card-value">{}</div><div>Total Incidents</div></div>
            <div class="card"><div class="card-value">{}</div><div>Total Downtime</div></div>
        </div>

        <h2>🌐 Monitor Performance</h2>
        <table><thead><tr><th>Monitor</th><th>URL</th><th>Uptime</th><th>Avg Response</th><th>Total Checks</th><th>Downtime</th></tr></thead><tbody>{}</tbody></table>

        <h2>🔴 Incident History</h2>
        <table><thead><tr><th>Date & Time</th><th>Monitor</th><th>Duration</th><th>Severity</th><th>Message</th><th>Status</th></tr></thead><tbody>{}</tbody></table>

        <h2>🔒 SSL Certificates</h2>
        <table><thead><tr><th>Domain</th><th>Status</th><th>Days Left</th><th>Expiry Date</th><th>Issuer</th></tr></thead><tbody>{}</tbody></table>

        <div class="footer">Generated by Basic Monitoring System</div>
    </div>
</body>
</html>"##,
        sla_bg_start, sla_bg_end,
        start_date, end_date,
        Utc::now().format("%Y-%m-%d %H:%M:%S"),
        sla.target_percent,
        sla.achieved_percent,
        sla.status,
        total_downtime_display,
        allowed_downtime_display,
        buffer_display,
        avg_uptime,
        avg_response,
        total_incidents,
        total_downtime_display,
        monitor_rows,
        incident_rows,
        ssl_rows,
    );

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .append_header(("Content-Disposition", "attachment; filename=\"monitoring_report.html\""))
        .body(html)
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

async fn monitor_report(
    _session: Session,
    _db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let monitor_id = path.into_inner();
    HttpResponse::Found()
        .append_header(("Location", format!("/reports?monitor={}", monitor_id)))
        .finish()
}
