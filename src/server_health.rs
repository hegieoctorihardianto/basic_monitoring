use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use serde::Serialize;
use sysinfo::{System, Disks, Networks};

// INCLUDE SEPERTI PHP
use crate::navbar;
use crate::sidebar;
use crate::footer;

#[derive(Serialize)]
struct ServerMetrics {
    cpu_usage: f32,
    cpu_cores: usize,
    memory_total: u64,
    memory_used: u64,
    memory_percent: f64,
    swap_total: u64,
    swap_used: u64,
    uptime: u64,
    hostname: String,
    os_version: String,
    kernel_version: String,
    disks: Vec<DiskInfo>,
    networks: Vec<NetworkInfo>,
    top_processes: Vec<ProcessInfo>,
}

#[derive(Serialize)]
struct DiskInfo {
    name: String,
    mount_point: String,
    total_space: u64,
    available_space: u64,
    used_space: u64,
    usage_percent: f64,
}

#[derive(Serialize)]
struct NetworkInfo {
    interface: String,
    received: u64,
    transmitted: u64,
}

#[derive(Serialize)]
struct ProcessInfo {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory: u64,
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/server")
            .route("", web::get().to(server_page))
            .route("/metrics", web::get().to(server_metrics_api))
    );
}

async fn server_page(session: Session) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => render_server_page(&email),
        _ => HttpResponse::Found()
            .append_header(("Location", "/"))
            .finish(),
    }
}

fn render_server_page(email: &str) -> HttpResponse {
    let mut html = String::new();

    html.push_str(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Server Health - Basic Monitoring</title>
    <link href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/sweetalert2@11"></script>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
        body { font-family: 'Inter', sans-serif; background-color: #f8fafc; }
        .sidebar-bg { background: linear-gradient(180deg, #1e293b 0%, #0f172a 100%); }
        .metric-card { transition: transform 0.2s ease; }
        .metric-card:hover { transform: translateY(-2px); }
        .progress-bar { height: 8px; border-radius: 4px; background: #e2e8f0; }
        .progress-fill { height: 8px; border-radius: 4px; transition: width 0.3s ease; }
        .bg-cpu { background: #3b82f6; }
        .bg-ram { background: #10b981; }
        .bg-disk { background: #f59e0b; }
        .loading-spinner { display: inline-block; width: 20px; height: 20px; border: 2px solid #e5e7eb; border-top-color: #3b82f6; border-radius: 50%; animation: spin 1s linear infinite; }
        @keyframes spin { to { transform: rotate(360deg); } }
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
                    <h1 class="text-2xl font-bold text-gray-800">🖥️ Server Health</h1>
                    <p class="text-gray-600">Real-time metrics of the server where this application runs</p>
                </div>
                <div>
                    <span class="px-3 py-1 bg-green-100 text-green-800 rounded-full text-sm" id="update-badge">
                        Live Updates
                    </span>
                </div>
            </div>

            <!-- CPU & Memory Row -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
                <!-- CPU Card -->
                <div class="bg-white rounded-lg shadow p-6 metric-card">
                    <div class="flex justify-between items-center mb-4">
                        <h3 class="text-lg font-semibold">🔥 CPU Usage</h3>
                        <span class="text-2xl font-bold text-blue-600" id="cpu-percent">0%</span>
                    </div>
                    <div class="progress-bar mb-2">
                        <div id="cpu-progress" class="progress-fill bg-cpu" style="width: 0%"></div>
                    </div>
                    <div class="flex justify-between text-sm text-gray-600">
                        <span>Cores: <span id="cpu-cores">0</span></span>
                        <span>Load: <span id="cpu-load">0%</span></span>
                    </div>
                </div>

                <!-- RAM Card -->
                <div class="bg-white rounded-lg shadow p-6 metric-card">
                    <div class="flex justify-between items-center mb-4">
                        <h3 class="text-lg font-semibold">📊 Memory (RAM)</h3>
                        <span class="text-2xl font-bold text-green-600" id="ram-percent">0%</span>
                    </div>
                    <div class="progress-bar mb-2">
                        <div id="ram-progress" class="progress-fill bg-ram" style="width: 0%"></div>
                    </div>
                    <div class="flex justify-between text-sm text-gray-600">
                        <span>Used: <span id="ram-used">0 GB</span></span>
                        <span>Total: <span id="ram-total">0 GB</span></span>
                    </div>
                    <div class="mt-2 text-xs text-gray-500">
                        Swap used: <span id="swap-used">0 MB</span>
                    </div>
                </div>
            </div>

            <!-- Disk Row -->
            <div class="bg-white rounded-lg shadow p-6 mb-6 metric-card">
                <h3 class="text-lg font-semibold mb-4">💾 Disk Usage</h3>
                <div id="disks-container" class="space-y-4">
                    <!-- Disks will be loaded here -->
                    <div class="text-center py-4">
                        <div class="loading-spinner mx-auto"></div>
                    </div>
                </div>
            </div>

            <!-- System Info & Network Row -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
                <!-- System Info -->
                <div class="bg-white rounded-lg shadow p-6">
                    <h3 class="text-lg font-semibold mb-4">🖥️ System Information</h3>
                    <div class="space-y-2 text-sm">
                        <div class="flex justify-between py-1 border-b">
                            <span class="text-gray-600">Hostname:</span>
                            <span class="font-medium" id="hostname">-</span>
                        </div>
                        <div class="flex justify-between py-1 border-b">
                            <span class="text-gray-600">OS:</span>
                            <span class="font-medium" id="os-version">-</span>
                        </div>
                        <div class="flex justify-between py-1 border-b">
                            <span class="text-gray-600">Kernel:</span>
                            <span class="font-medium" id="kernel-version">-</span>
                        </div>
                        <div class="flex justify-between py-1 border-b">
                            <span class="text-gray-600">Uptime:</span>
                            <span class="font-medium" id="uptime">-</span>
                        </div>
                    </div>
                </div>

                <!-- Network -->
                <div class="bg-white rounded-lg shadow p-6">
                    <h3 class="text-lg font-semibold mb-4">🌐 Network Traffic</h3>
                    <div id="networks-container" class="space-y-3">
                        <!-- Networks will be loaded here -->
                        <div class="text-center py-4">
                            <div class="loading-spinner mx-auto"></div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Top Processes -->
            <div class="bg-white rounded-lg shadow p-6">
                <h3 class="text-lg font-semibold mb-4">📋 Top Processes (by CPU)</h3>
                <div class="overflow-x-auto">
                    <table class="min-w-full">
                        <thead>
                            <tr class="bg-gray-50">
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500">PID</th>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500">Process</th>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500">CPU %</th>
                                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500">Memory (MB)</th>
                            </tr>
                        </thead>
                        <tbody id="processes-container">
                            <tr>
                                <td colspan="4" class="px-4 py-8 text-center">
                                    <div class="loading-spinner mx-auto"></div>
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
        function formatBytes(bytes) {
            if (bytes === 0) return '0 B';
            const k = 1024;
            const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
        }

        function formatUptime(seconds) {
            const days = Math.floor(seconds / 86400);
            const hours = Math.floor((seconds % 86400) / 3600);
            const minutes = Math.floor((seconds % 3600) / 60);

            let result = '';
            if (days > 0) result += days + 'd ';
            if (hours > 0) result += hours + 'h ';
            result += minutes + 'm';
            return result;
        }

        function loadServerMetrics() {
            fetch('/server/metrics')
                .then(response => response.json())
                .then(data => {
                    // CPU
                    document.getElementById('cpu-percent').textContent = data.cpu_usage.toFixed(1) + '%';
                    document.getElementById('cpu-progress').style.width = data.cpu_usage + '%';
                    document.getElementById('cpu-cores').textContent = data.cpu_cores;
                    document.getElementById('cpu-load').textContent = data.cpu_usage.toFixed(1) + '%';

                    // RAM
                    const ramPercent = data.memory_percent;
                    document.getElementById('ram-percent').textContent = ramPercent.toFixed(1) + '%';
                    document.getElementById('ram-progress').style.width = ramPercent + '%';
                    document.getElementById('ram-used').textContent = formatBytes(data.memory_used);
                    document.getElementById('ram-total').textContent = formatBytes(data.memory_total);
                    document.getElementById('swap-used').textContent = formatBytes(data.swap_used);

                    // System Info
                    document.getElementById('hostname').textContent = data.hostname;
                    document.getElementById('os-version').textContent = data.os_version;
                    document.getElementById('kernel-version').textContent = data.kernel_version;
                    document.getElementById('uptime').textContent = formatUptime(data.uptime);

                    // Disks
                    let disksHtml = '';
                    data.disks.forEach(disk => {
                        disksHtml += `
                            <div class="mb-3">
                                <div class="flex justify-between text-sm mb-1">
                                    <span class="font-medium">${disk.name} (${disk.mount_point})</span>
                                    <span>${formatBytes(disk.used_space)} / ${formatBytes(disk.total_space)} (${disk.usage_percent.toFixed(1)}%)</span>
                                </div>
                                <div class="progress-bar">
                                    <div class="progress-fill bg-disk" style="width: ${disk.usage_percent}%"></div>
                                </div>
                            </div>
                        `;
                    });
                    document.getElementById('disks-container').innerHTML = disksHtml;

                    // Networks
                    let networksHtml = '';
                    data.networks.forEach(net => {
                        networksHtml += `
                            <div class="flex justify-between text-sm py-1 border-b">
                                <span class="font-medium">${net.interface}</span>
                                <span>⬇️ ${formatBytes(net.received)}/s | ⬆️ ${formatBytes(net.transmitted)}/s</span>
                            </div>
                        `;
                    });
                    document.getElementById('networks-container').innerHTML = networksHtml;

                    // Processes
                    let processesHtml = '';
                    data.top_processes.forEach(proc => {
                        processesHtml += `
                            <tr class="border-b">
                                <td class="px-4 py-2">${proc.pid}</td>
                                <td class="px-4 py-2 font-medium">${proc.name}</td>
                                <td class="px-4 py-2">${proc.cpu_usage.toFixed(1)}%</td>
                                <td class="px-4 py-2">${(proc.memory / 1024 / 1024).toFixed(0)} MB</td>
                            </tr>
                        `;
                    });
                    document.getElementById('processes-container').innerHTML = processesHtml;

                    // Update badge
                    document.getElementById('update-badge').innerHTML = '✅ Updated ' + new Date().toLocaleTimeString();
                })
                .catch(error => {
                    console.error('Error loading server metrics:', error);
                    document.getElementById('update-badge').innerHTML = '❌ Update failed';
                });
        }

        // Load every 5 seconds
        document.addEventListener('DOMContentLoaded', function() {
            loadServerMetrics();
            setInterval(loadServerMetrics, 5000);
        });
    </script>
    </body>
    </html>
    "##);

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

async fn server_metrics_api(
    session: Session,
) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(_)) => {
            let mut sys = System::new_all();
            sys.refresh_all();

            // CPU
            let cpu_usage = sys.global_cpu_info().cpu_usage();
            let cpu_cores = sys.cpus().len();

            // Memory
            let memory_total = sys.total_memory();
            let memory_used = sys.used_memory();
            let memory_percent = (memory_used as f64 / memory_total as f64) * 100.0;

            // Swap
            let swap_total = sys.total_swap();
            let swap_used = sys.used_swap();

            // Disks
            let disks = Disks::new_with_refreshed_list();
            let disk_infos: Vec<DiskInfo> = disks.list().iter().map(|disk| {
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total - available;
                let usage_percent = (used as f64 / total as f64) * 100.0;

                DiskInfo {
                    name: disk.name().to_string_lossy().to_string(),
                    mount_point: disk.mount_point().to_string_lossy().to_string(),
                    total_space: total,
                    available_space: available,
                    used_space: used,
                    usage_percent,
                }
            }).collect();

            // Networks
            let networks = Networks::new_with_refreshed_list();
            let network_infos: Vec<NetworkInfo> = networks.list().iter().map(|(name, data)| {
                NetworkInfo {
                    interface: name.clone(),
                    received: data.received(),
                    transmitted: data.transmitted(),
                }
            }).collect();

            // Top processes (by CPU)
            let mut processes: Vec<ProcessInfo> = sys.processes().iter()
                .map(|(pid, process)| ProcessInfo {
                    pid: pid.as_u32(),
                    name: process.name().to_string(),  // ✅ GANTI INI!
                    cpu_usage: process.cpu_usage(),
                    memory: process.memory(),
                })
                .collect();

            processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
            processes.truncate(10);

            // System info
            let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());
            let os_version = System::os_version().unwrap_or_else(|| "unknown".to_string());
            let kernel_version = System::kernel_version().unwrap_or_else(|| "unknown".to_string());
            let uptime = System::uptime();

            let metrics = ServerMetrics {
                cpu_usage,
                cpu_cores,
                memory_total,
                memory_used,
                memory_percent,
                swap_total,
                swap_used,
                uptime,
                hostname,
                os_version,
                kernel_version,
                disks: disk_infos,
                networks: network_infos,
                top_processes: processes,
            };

            HttpResponse::Ok().json(metrics)
        }
        _ => HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "Not authenticated"
        }))
    }
}
