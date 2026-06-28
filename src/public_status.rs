// src/public_status.rs
use actix_web::{web, HttpResponse, Responder};
use sqlx::{SqlitePool, Row};
use serde::Serialize;
use chrono::Duration;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/status")
            .route("/{id}", web::get().to(public_status_page))
            .route("/{id}/data", web::get().to(public_status_data))
            .route("/{id}/badge.svg", web::get().to(public_status_badge))
    );
}

#[derive(Serialize)]
struct StatusData {
    monitor_name: String,
    monitor_url: String,
    current_status: String,
    last_check: Option<String>,
    uptime_24h: f64,
    uptime_7d: f64,
    uptime_30d: f64,
    avg_response_24h: i64,
    avg_response_7d: i64,
    avg_response_30d: i64,
    total_checks: i64,
    history: Vec<DailyHistory>,
    recent_alerts: Vec<AlertHistory>,
}

#[derive(Serialize)]
struct DailyHistory {
    date: String,
    uptime_percent: f64,
    avg_response: i64,
}

#[derive(Serialize)]
struct AlertHistory {
    time: String,
    severity: String,
    message: String,
    duration: Option<String>,
}

// ==================== PUBLIC STATUS PAGE ====================

async fn public_status_page(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let monitor_id = path.into_inner();

    // Cek apakah monitor ada
    let monitor = sqlx::query!(
        "SELECT id, name, target_url FROM monitors WHERE id = ?",
        monitor_id
    )
    .fetch_optional(db_pool.get_ref())
    .await;

    let monitor = match monitor {
        Ok(Some(m)) => m,
        _ => {
            return HttpResponse::NotFound().body(r##"<!DOCTYPE html>
<html>
<head><title>404 - Not Found</title></head>
<body>
    <h1>404 - Monitor Not Found</h1>
    <p>The requested status page does not exist.</p>
</body>
</html>"##);
        }
    };

    render_status_page(&monitor.name, &monitor.target_url, monitor_id).await
}

async fn render_status_page(monitor_name: &str, monitor_url: &str, monitor_id: i64) -> HttpResponse {
    let html = format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} Status - Basic Monitoring</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/sweetalert2@11"></script>
    <link href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css" rel="stylesheet">
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
        body {{ font-family: 'Inter', sans-serif; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); min-height: 100vh; }}
        .glass-card {{ background: rgba(255, 255, 255, 0.95); backdrop-filter: blur(10px); border-radius: 1rem; box-shadow: 0 20px 40px rgba(0,0,0,0.1); }}
        .status-badge {{ padding: 0.25rem 0.75rem; border-radius: 9999px; font-size: 0.875rem; font-weight: 600; display: inline-block; }}
        .status-up {{ background-color: #10b981; color: white; }}
        .status-down {{ background-color: #ef4444; color: white; }}
        .status-unknown {{ background-color: #6b7280; color: white; }}
        .stat-card {{ background: white; border-radius: 0.75rem; padding: 1.25rem; box-shadow: 0 4px 6px rgba(0,0,0,0.05); transition: all 0.2s; }}
        .stat-card:hover {{ transform: translateY(-2px); box-shadow: 0 10px 15px rgba(0,0,0,0.1); }}
        .progress-bar {{ height: 8px; background: #e2e8f0; border-radius: 4px; overflow: hidden; }}
        .progress-fill {{ height: 100%; transition: width 0.3s ease; }}
        .bg-uptime-good {{ background: #10b981; }}
        .bg-uptime-warning {{ background: #f59e0b; }}
        .bg-uptime-bad {{ background: #ef4444; }}
        .embed-code {{ background: #1e293b; color: #e2e8f0; padding: 1rem; border-radius: 0.5rem; font-family: monospace; font-size: 0.875rem; }}
        .copy-btn {{ cursor: pointer; transition: all 0.2s; }}
        .copy-btn:hover {{ background: #f1f5f9; }}

        /* Batasi tinggi chart */
        canvas {{
            max-height: 250px;
            width: 100% !important;
        }}

        /* Container chart */
        .chart-container {{
            position: relative;
            height: 250px;
            width: 100%;
        }}

        /* Daily uptime container */
        #daily-uptime {{
            max-height: 400px;
            overflow-y: auto;
            padding-right: 10px;
        }}

        /* Scrollbar styling */
        #daily-uptime::-webkit-scrollbar {{
            width: 6px;
        }}
        #daily-uptime::-webkit-scrollbar-track {{
            background: #f1f1f1;
            border-radius: 10px;
        }}
        #daily-uptime::-webkit-scrollbar-thumb {{
            background: #888;
            border-radius: 10px;
        }}
        #daily-uptime::-webkit-scrollbar-thumb:hover {{
            background: #555;
        }}
    </style>
</head>
<body class="p-4">
    <div class="max-w-6xl mx-auto">
        <!-- Header with Glass Effect -->
        <div class="glass-card p-8 mb-6">
            <div class="flex flex-col md:flex-row justify-between items-start md:items-center">
                <div>
                    <h1 class="text-3xl font-bold text-gray-800 mb-2">{}</h1>
                    <a href="{}" target="_blank" class="text-blue-600 hover:text-blue-800 flex items-center">
                        <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path>
                        </svg>
                        {}
                    </a>
                </div>
                <div class="mt-4 md:mt-0 flex items-center space-x-4">
                    <div id="status-badge" class="status-badge status-unknown">Loading...</div>
                    <button onclick="copyEmbedCode()" class="px-4 py-2 bg-gray-100 rounded-lg hover:bg-gray-200 transition flex items-center">
                        <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                        </svg>
                        Embed
                    </button>
                </div>
            </div>
        </div>

        <!-- Stats Cards -->
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6" id="stats-container">
            <div class="stat-card animate-pulse">
                <div class="h-4 bg-gray-200 rounded w-1/2 mb-2"></div>
                <div class="h-8 bg-gray-300 rounded w-3/4"></div>
            </div>
            <div class="stat-card animate-pulse">
                <div class="h-4 bg-gray-200 rounded w-1/2 mb-2"></div>
                <div class="h-8 bg-gray-300 rounded w-3/4"></div>
            </div>
            <div class="stat-card animate-pulse">
                <div class="h-4 bg-gray-200 rounded w-1/2 mb-2"></div>
                <div class="h-8 bg-gray-300 rounded w-3/4"></div>
            </div>
            <div class="stat-card animate-pulse">
                <div class="h-4 bg-gray-200 rounded w-1/2 mb-2"></div>
                <div class="h-8 bg-gray-300 rounded w-3/4"></div>
            </div>
        </div>

        <!-- Response Time Chart -->
        <div class="glass-card p-6 mb-6">
            <h2 class="text-xl font-bold text-gray-800 mb-4">📈 Response Time (30 Days)</h2>
            <div class="chart-container">
                <canvas id="responseChart"></canvas>
            </div>
        </div>

        <!-- Daily Uptime -->
        <div class="glass-card p-6 mb-6">
            <h2 class="text-xl font-bold text-gray-800 mb-4">📊 Daily Uptime (Last 30 Days)</h2>
            <div id="daily-uptime" class="space-y-2">
                <div class="text-center py-8 text-gray-500">Loading...</div>
            </div>
        </div>

        <!-- Alert History -->
        <div class="glass-card p-6">
            <h2 class="text-xl font-bold text-gray-800 mb-4">🔔 Recent Alerts</h2>
            <div id="alert-history" class="space-y-3 max-h-96 overflow-y-auto pr-2">
                <div class="text-center py-8 text-gray-500">Loading...</div>
            </div>
        </div>

        <!-- Embed Modal -->
        <div id="embed-modal" class="fixed inset-0 bg-black bg-opacity-50 hidden items-center justify-center z-50">
            <div class="bg-white rounded-lg p-6 max-w-2xl w-full mx-4">
                <h3 class="text-xl font-bold mb-4">📋 Embed Status Badge</h3>

                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-2">HTML</label>
                    <div class="embed-code text-sm mb-2" id="html-code">&lt;iframe src="/status/{}" width="100%" height="400"&gt;&lt;/iframe&gt;</div>
                    <button onclick="copyToClipboard('html-code')" class="text-sm text-blue-600 hover:text-blue-800">Copy</button>
                </div>

                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-2">Markdown</label>
                    <div class="embed-code text-sm mb-2" id="md-code">[![Status]({}/badge.svg)]({}/status/{})</div>
                    <button onclick="copyToClipboard('md-code')" class="text-sm text-blue-600 hover:text-blue-800">Copy</button>
                </div>

                <div class="flex justify-end">
                    <button onclick="closeModal()" class="px-4 py-2 bg-gray-200 rounded-lg hover:bg-gray-300">Close</button>
                </div>
            </div>
        </div>
    </div>

    <script>
        let monitorId = {};
        let chart;

        function formatDate(dateStr) {{
            const d = new Date(dateStr);
            return d.toLocaleDateString('en-US', {{ month: 'short', day: 'numeric' }});
        }}

        function getStatusClass(status) {{
            switch(status) {{
                case 'up': return 'status-up';
                case 'down': return 'status-down';
                default: return 'status-unknown';
            }}
        }}

        function getStatusText(status) {{
            switch(status) {{
                case 'up': return '🟢 Operational';
                case 'down': return '🔴 Down';
                default: return '⚪ Unknown';
            }}
        }}

        function loadData() {{
            fetch(`/status/${{monitorId}}/data`)
                .then(res => res.json())
                .then(data => {{
                    // Update status badge
                    const badge = document.getElementById('status-badge');
                    badge.className = `status-badge ${{getStatusClass(data.current_status)}}`;
                    badge.textContent = getStatusText(data.current_status);

                    // Update stats
                    document.getElementById('stats-container').innerHTML = `
                        <div class="stat-card">
                            <div class="text-sm text-gray-600">Uptime (24h)</div>
                            <div class="text-2xl font-bold">${{data.uptime_24h.toFixed(2)}}%</div>
                        </div>
                        <div class="stat-card">
                            <div class="text-sm text-gray-600">Uptime (7d)</div>
                            <div class="text-2xl font-bold">${{data.uptime_7d.toFixed(2)}}%</div>
                        </div>
                        <div class="stat-card">
                            <div class="text-sm text-gray-600">Uptime (30d)</div>
                            <div class="text-2xl font-bold">${{data.uptime_30d.toFixed(2)}}%</div>
                        </div>
                        <div class="stat-card">
                            <div class="text-sm text-gray-600">Total Checks</div>
                            <div class="text-2xl font-bold">${{data.total_checks}}</div>
                        </div>
                    `;

                    // Update chart
                    if (chart) chart.destroy();
                    const ctx = document.getElementById('responseChart').getContext('2d');
                    chart = new Chart(ctx, {{
                        type: 'line',
                        data: {{
                            labels: data.history.map(h => formatDate(h.date)),
                            datasets: [{{
                                label: 'Response Time (ms)',
                                data: data.history.map(h => h.avg_response),
                                borderColor: '#3b82f6',
                                backgroundColor: 'rgba(59, 130, 246, 0.1)',
                                tension: 0.4,
                                fill: true
                            }}]
                        }},
                        options: {{
                            responsive: true,
                            maintainAspectRatio: false,
                            plugins: {{
                                legend: {{ display: false }}
                            }},
                            scales: {{
                                y: {{
                                    beginAtZero: true,
                                    title: {{ display: true, text: 'ms' }}
                                }}
                            }}
                        }}
                    }});

                    // Update daily uptime
                    let uptimeHtml = '';
                    if (data.history.length === 0) {{
                        uptimeHtml = '<div class="text-center py-4 text-gray-500">No data available</div>';
                    }} else {{
                        data.history.slice(0, 30).forEach(day => {{
                            let colorClass = 'bg-uptime-good';
                            if (day.uptime_percent < 99) colorClass = 'bg-uptime-warning';
                            if (day.uptime_percent < 95) colorClass = 'bg-uptime-bad';

                            uptimeHtml += `
                                <div class="flex items-center text-sm">
                                    <div class="w-24 font-medium">${{formatDate(day.date)}}</div>
                                    <div class="flex-1 mx-4">
                                        <div class="progress-bar">
                                            <div class="progress-fill ${{colorClass}}" style="width:${{day.uptime_percent}}%"></div>
                                        </div>
                                    </div>
                                    <div class="w-16 text-right font-mono">${{day.uptime_percent.toFixed(2)}}%</div>
                                    <div class="w-16 text-right text-gray-500 font-mono">${{day.avg_response}}ms</div>
                                </div>
                            `;
                        }});
                    }}
                    document.getElementById('daily-uptime').innerHTML = uptimeHtml;

                    // Update alert history
                    let alertHtml = '';
                    if (data.recent_alerts.length === 0) {{
                        alertHtml = '<div class="text-center py-8 text-gray-500">No recent alerts</div>';
                    }} else {{
                        data.recent_alerts.forEach(alert => {{
                            let severityColor = alert.severity === 'critical' ? 'bg-red-100 text-red-800' :
                                              alert.severity === 'warning' ? 'bg-yellow-100 text-yellow-800' :
                                              'bg-blue-100 text-blue-800';
                            alertHtml += `
                                <div class="bg-white rounded-lg p-4 border border-gray-200 hover:shadow-md transition">
                                    <div class="flex justify-between items-start">
                                        <span class="px-2 py-1 rounded text-xs font-semibold ${{severityColor}}">${{alert.severity}}</span>
                                        <span class="text-xs text-gray-500">${{alert.time}}</span>
                                    </div>
                                    <p class="mt-2 text-gray-800">${{alert.message}}</p>
                                    ${{alert.duration ? `<p class="text-sm text-gray-500 mt-1">Duration: ${{alert.duration}}</p>` : ''}}
                                </div>
                            `;
                        }});
                    }}
                    document.getElementById('alert-history').innerHTML = alertHtml;
                }})
                .catch(error => {{
                    console.error('Error loading status:', error);
                }});
        }}

        function copyToClipboard(elementId) {{
            const text = document.getElementById(elementId).textContent;
            navigator.clipboard.writeText(text);
            Swal.fire({{
                icon: 'success',
                title: 'Copied!',
                text: 'Code copied to clipboard',
                timer: 1500,
                showConfirmButton: false
            }});
        }}

        function copyEmbedCode() {{
            document.getElementById('embed-modal').classList.remove('hidden');
            document.getElementById('embed-modal').classList.add('flex');
        }}

        function closeModal() {{
            document.getElementById('embed-modal').classList.add('hidden');
            document.getElementById('embed-modal').classList.remove('flex');
        }}

        document.addEventListener('DOMContentLoaded', function() {{
            loadData();
            setInterval(loadData, 30000);
        }});

        document.getElementById('embed-modal').addEventListener('click', function(e) {{
            if (e.target === this) closeModal();
        }});
    </script>
</body>
</html>"##, monitor_name, monitor_name, monitor_url, monitor_url, monitor_id, monitor_id, monitor_id, monitor_id, monitor_id);

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

// ==================== API ENDPOINT ====================

async fn public_status_data(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let monitor_id = path.into_inner();
    let pool = db_pool.get_ref();

    // Get monitor info
    let monitor = sqlx::query!(
        "SELECT name, target_url FROM monitors WHERE id = ?",
        monitor_id
    )
    .fetch_optional(pool)
    .await;

    let (monitor_name, monitor_url) = match monitor {
        Ok(Some(m)) => (m.name, m.target_url),
        _ => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Monitor not found"
            }));
        }
    };

    // Get current status
    let current = sqlx::query!(
        "SELECT status FROM check_results WHERE monitor_id = ? ORDER BY checked_at DESC LIMIT 1",
        monitor_id
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let current_status = current.map(|r| r.status).unwrap_or_else(|| "unknown".to_string());

    // Get uptime stats
    let uptime_24h = get_uptime(pool, monitor_id, 24).await;
    let uptime_7d = get_uptime(pool, monitor_id, 168).await;
    let uptime_30d = get_uptime(pool, monitor_id, 720).await;

    // Get avg response times
    let avg_24h = get_avg_response(pool, monitor_id, 24).await;
    let avg_7d = get_avg_response(pool, monitor_id, 168).await;
    let avg_30d = get_avg_response(pool, monitor_id, 720).await;

    // Get total checks
    let total_checks = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM check_results WHERE monitor_id = ?",
        monitor_id
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Get daily history
    let history = get_daily_history(pool, monitor_id).await;

    // Get recent alerts
    let alerts = sqlx::query(
        "SELECT CAST(triggered_at AS TEXT) as triggered_at, severity, message, CAST(resolved_at AS TEXT) as resolved_at
         FROM alerts WHERE monitor_id = ?
         ORDER BY triggered_at DESC LIMIT 10"
    )
    .bind(monitor_id)
    .fetch_all(pool)
    .await
    .unwrap_or(vec![]);

    let mut recent_alerts = Vec::new();
    for row in alerts {
        let triggered_at: String = row.get(0);
        let severity: String = row.get(1);
        let message: String = row.get(2);
        let resolved_at: Option<String> = row.get(3);

        let duration = if let (Ok(triggered), Some(Ok(resolved))) = (
            chrono::NaiveDateTime::parse_from_str(&triggered_at, "%Y-%m-%d %H:%M:%S"),
            resolved_at.as_ref().map(|r| chrono::NaiveDateTime::parse_from_str(r, "%Y-%m-%d %H:%M:%S"))
        ) {
            let diff: Duration = resolved.signed_duration_since(triggered);
            Some(format!("{}m", diff.num_minutes()))
        } else {
            None
        };

        recent_alerts.push(AlertHistory {
            time: triggered_at,
            severity,
            message,
            duration,
        });
    }

    HttpResponse::Ok().json(serde_json::json!({
        "monitor_name": monitor_name,
        "monitor_url": monitor_url,
        "current_status": current_status,
        "uptime_24h": uptime_24h,
        "uptime_7d": uptime_7d,
        "uptime_30d": uptime_30d,
        "avg_response_24h": avg_24h,
        "avg_response_7d": avg_7d,
        "avg_response_30d": avg_30d,
        "total_checks": total_checks,
        "history": history,
        "recent_alerts": recent_alerts,
    }))
}

async fn get_uptime(pool: &SqlitePool, monitor_id: i64, hours: i64) -> f64 {
    let result = sqlx::query(
        "SELECT
            COUNT(*) as total,
            SUM(CASE WHEN status = 'up' THEN 1 ELSE 0 END) as up
         FROM check_results
         WHERE monitor_id = ? AND checked_at >= datetime('now', ? || ' hours')"
    )
    .bind(monitor_id)
    .bind(format!("-{}", hours))
    .fetch_one(pool)
    .await;

    match result {
        Ok(row) => {
            let total: i64 = row.get(0);
            let up: i64 = row.get(1);
            if total > 0 { (up as f64 / total as f64) * 100.0 } else { 100.0 }
        }
        Err(_) => 100.0,
    }
}

async fn get_avg_response(pool: &SqlitePool, monitor_id: i64, hours: i64) -> i64 {
    let result = sqlx::query_scalar::<_, Option<f64>>(
        "SELECT AVG(response_time) FROM check_results
         WHERE monitor_id = ? AND checked_at >= datetime('now', ? || ' hours') AND response_time IS NOT NULL"
    )
    .bind(monitor_id)
    .bind(format!("-{}", hours))
    .fetch_one(pool)
    .await;

    match result {
        Ok(Some(avg)) => avg.round() as i64,
        _ => 0,
    }
}

async fn get_daily_history(pool: &SqlitePool, monitor_id: i64) -> Vec<DailyHistory> {
    let rows = sqlx::query(
        "SELECT
            date(checked_at) as day,
            AVG(response_time) as avg_resp,
            COUNT(*) as total,
            SUM(CASE WHEN status = 'up' THEN 1 ELSE 0 END) as up
         FROM check_results
         WHERE monitor_id = ? AND checked_at >= datetime('now', '-30 days')
         GROUP BY date(checked_at)
         ORDER BY day DESC"
    )
    .bind(monitor_id)
    .fetch_all(pool)
    .await
    .unwrap_or(vec![]);

    let mut history = Vec::new();
    for row in rows {
        let day: String = row.get("day");
        let avg: f64 = row.get("avg_resp");
        let total: i64 = row.get("total");
        let up: i64 = row.get("up");
        let uptime = if total > 0 { (up as f64 / total as f64) * 100.0 } else { 100.0 };

        history.push(DailyHistory {
            date: day,
            uptime_percent: uptime,
            avg_response: avg.round() as i64,
        });
    }
    history
}

// ==================== BADGE GENERATOR ====================

async fn public_status_badge(
    db_pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> impl Responder {
    let monitor_id = path.into_inner();
    let uptime = get_uptime(db_pool.get_ref(), monitor_id, 720).await;

    let color = if uptime >= 99.9 { "#4c1" }
                else if uptime >= 99.0 { "#dfb317" }
                else { "#e05d44" };

    let text = format!("{:.2}%", uptime);

    let svg = format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="20">
        <linearGradient id="b" x2="0" y2="100%">
            <stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
            <stop offset="1" stop-opacity=".1"/>
        </linearGradient>
        <mask id="a">
            <rect width="96" height="20" rx="3" fill="#fff"/>
        </mask>
        <g mask="url(#a)">
            <path fill="#555" d="M0 0h51v20H0z"/>
            <path fill="{}" d="M51 0h45v20H51z"/>
            <path fill="url(#b)" d="M0 0h96v20H0z"/>
        </g>
        <g fill="#fff" text-anchor="middle" font-family="DejaVu Sans,Verdana,Geneva,sans-serif" font-size="11">
            <text x="25.5" y="15" fill="#010101" fill-opacity=".3">uptime</text>
            <text x="25.5" y="14">uptime</text>
            <text x="72.5" y="15" fill="#010101" fill-opacity=".3">{}</text>
            <text x="72.5" y="14">{}</text>
        </g>
    </svg>"##, color, text, text);

    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(svg)
}
