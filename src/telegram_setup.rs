use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use sqlx::SqlitePool;

// INCLUDE SEPERTI PHP
use crate::navbar;
use crate::sidebar;
use crate::footer;
use crate::telegram::TelegramNotifier;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/telegram-setup")
            .route("", web::get().to(telegram_setup_page))
            .route("/save", web::post().to(save_telegram_config))
            .route("/test", web::post().to(test_telegram_config))
            .route("/send-deeplink", web::post().to(send_deeplink))
    );
}

async fn telegram_setup_page(session: Session, db_pool: web::Data<SqlitePool>) -> impl Responder {
    match session.get::<String>("email") {
        Ok(Some(email)) => {
            match session.get::<i32>("user_id") {
                Ok(Some(user_id)) => {
                    render_telegram_setup(&email, user_id, &db_pool).await
                }
                _ => HttpResponse::Found()
                    .append_header(("Location", "/"))
                    .finish(),
            }
        }
        _ => HttpResponse::Found()
            .append_header(("Location", "/"))
            .finish(),
    }
}

async fn render_telegram_setup(email: &str, user_id: i32, db_pool: &SqlitePool) -> HttpResponse {
    // Cek apakah sudah ada konfigurasi
    let existing_config = TelegramNotifier::get_user_config(db_pool, user_id).await.ok().flatten();

    let mut html = String::new();

    html.push_str(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Telegram Setup - Basic Monitoring</title>
    <link href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/sweetalert2@11"></script>
    <style>
        .bg-telegram {
            background-color: #0088cc;
        }
        .bg-telegram:hover {
            background-color: #0077b3;
        }
        .step-card {
            transition: all 0.3s ease;
        }
        .step-card:hover {
            transform: translateY(-2px);
            box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.1);
        }
        .copy-btn {
            cursor: pointer;
            transition: all 0.2s ease;
        }
        .copy-btn:hover {
            background-color: #f1f5f9;
        }
        .copy-btn.copied {
            background-color: #10b981;
            color: white;
        }
    </style>
</head>
<body>
"##);

    html.push_str(&navbar::render(email));
    html.push_str(r#"<div class="flex">"#);
    html.push_str(&sidebar::render());

    // Tentukan nilai default form
    let default_chat_id = if let Some(config) = &existing_config {
        config.chat_id.clone()
    } else {
        "".to_string()
    };

    let default_enabled = if let Some(config) = &existing_config {
        config.enabled
    } else {
        false
    };

    html.push_str(r##"
    <!-- Main Content -->
    <div class="flex-1 p-6">
        <div class="max-w-4xl mx-auto">
            <div class="bg-white rounded-xl shadow-lg p-8">
                <h1 class="text-3xl font-bold text-slate-800 mb-2">🤖 Telegram Notification Setup</h1>
                <p class="text-slate-600 mb-8">Configure bot to receive monitoring alerts</p>

                <!-- Progress Steps -->
                <div class="mb-10">
                    <h2 class="text-xl font-bold text-slate-800 mb-4">📋 Setup Steps</h2>
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                        <div class="step-card bg-blue-50 p-5 rounded-xl border border-blue-200">
                            <div class="flex items-center mb-3">
                                <div class="w-8 h-8 bg-blue-500 rounded-full flex items-center justify-center mr-3">
                                    <span class="text-white font-bold">1</span>
                                </div>
                                <h3 class="font-bold text-blue-800">Get Chat ID</h3>
                            </div>
                            <p class="text-blue-700 text-sm">Use @userinfobot on Telegram</p>
                        </div>

                        <div class="step-card bg-green-50 p-5 rounded-xl border border-green-200">
                            <div class="flex items-center mb-3">
                                <div class="w-8 h-8 bg-green-500 rounded-full flex items-center justify-center mr-3">
                                    <span class="text-white font-bold">2</span>
                                </div>
                                <h3 class="font-bold text-green-800">Activate Bot</h3>
                            </div>
                            <p class="text-green-700 text-sm">Send /start to @my_monitor_core_bot</p>
                        </div>

                        <div class="step-card bg-purple-50 p-5 rounded-xl border border-purple-200">
                            <div class="flex items-center mb-3">
                                <div class="w-8 h-8 bg-purple-500 rounded-full flex items-center justify-center mr-3">
                                    <span class="text-white font-bold">3</span>
                                </div>
                                <h3 class="font-bold text-purple-800">Test & Save</h3>
                            </div>
                            <p class="text-purple-700 text-sm">Test configuration and save</p>
                        </div>
                    </div>
                </div>

                <!-- STEP 1: Dapatkan Chat ID -->
                <div class="mb-8 p-6 bg-gradient-to-r from-blue-50 to-indigo-50 rounded-xl border border-blue-200">
                    <div class="flex items-center mb-4">
                        <div class="w-10 h-10 bg-blue-500 rounded-full flex items-center justify-center mr-4">
                            <span class="text-white font-bold text-lg">1</span>
                        </div>
                        <div>
                            <h3 class="font-bold text-blue-800 text-lg">GET YOUR CHAT ID</h3>
                            <p class="text-blue-700">Use @userinfobot on Telegram</p>
                        </div>
                    </div>

                    <div class="grid grid-cols-1">
                        <div class="bg-white rounded-lg border border-blue-100">
                            <h4 class="font-bold text-blue-800 mb-3">How to Get Chat ID:</h4>
                            <ol class="list-decimal pl-5 space-y-2 text-blue-700">
                                <li>Open Telegram on your phone/computer</li>
                                <li>Search for bot: <code class="bg-blue-100 px-2 py-1 rounded">@userinfobot</code></li>
                                <li>Click "Start" or send <code>/start</code></li>
                                <li>Bot will reply with <strong>"Your ID:"</strong></li>
                                <li>Copy that ID number</li>
                            </ol>

                            <div class="mt-4">
                                <a href="https://t.me/userinfobot" target="_blank"
                                   class="inline-flex items-center px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm">
                                    <svg class="w-4 h-4 mr-2" fill="currentColor" viewBox="0 0 24 24">
                                        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm4.64 6.8c-.15 1.58-.8 5.42-1.13 7.19-.14.75-.42 1-.68 1.03-.58.05-1.02-.38-1.58-.75-.88-.58-1.38-.94-2.23-1.5-.99-.65-.35-1.01.22-1.59.15-.15 2.71-2.48 2.76-2.69.01-.03.01-.14-.06-.2-.07-.06-.17-.04-.24-.02-.1.02-1.69 1.09-4.78 3.2-.45.31-.86.46-1.23.45-.41-.01-1.2-.23-1.79-.42-.72-.23-1.3-.36-1.25-.76.02-.18.27-.36.75-.55 2.89-1.26 4.83-2.1 5.82-2.51 2.78-1.16 3.36-1.36 3.73-1.36.08 0 .27.02.39.12.1.08.13.19.14.27-.01.06.01.24 0 .38z"/>
                                    </svg>
                                    Open @userinfobot on Telegram
                                </a>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- STEP 2: Aktifkan Bot -->
                <div class="mb-8 p-6 bg-gradient-to-r from-green-50 to-emerald-50 rounded-xl border border-green-200">
                    <div class="flex items-center mb-4">
                        <div class="w-10 h-10 bg-green-500 rounded-full flex items-center justify-center mr-4">
                            <span class="text-white font-bold text-lg">2</span>
                        </div>
                        <div>
                            <h3 class="font-bold text-green-800 text-lg">ACTIVATE MONITORING BOT</h3>
                            <p class="text-green-700">Send /start to @my_monitor_core_bot</p>
                        </div>
                    </div>

                    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                        <div class="bg-white p-5 rounded-lg border border-green-100">
                            <h4 class="font-bold text-green-800 mb-3">Manual Method:</h4>
                            <ol class="list-decimal pl-5 space-y-2 text-green-700">
                                <li>Search for bot: <code class="bg-green-100 px-2 py-1 rounded">@my_monitor_core_bot</code></li>
                                <li>Send message: <code class="bg-green-100 px-2 py-1 rounded font-bold">/start</code></li>
                                <li>Send another message: <code class="bg-green-100 px-2 py-1 rounded">hello</code></li>
                                <li>Bot is ready to receive notifications</li>
                            </ol>

                            <div class="mt-4 p-3 bg-yellow-50 border border-yellow-200 rounded">
                                <p class="text-yellow-800 text-sm">
                                    <strong>IMPORTANT:</strong> After <code>/start</code>, you must send a regular message (hello/test/ok) for the bot to activate!
                                </p>
                            </div>
                        </div>

                        <div class="bg-white p-5 rounded-lg border border-green-100">
                            <h4 class="font-bold text-green-800 mb-3">Automatic Method (Recommended):</h4>
                            <p class="text-green-700 mb-3">Use Deep Link for 1-click activation:</p>

                            <div class="bg-gray-800 text-white p-4 rounded-lg">
                                <div class="flex justify-between items-center">
                                    <code class="text-green-300 text-sm break-all">
                                        https://t.me/my_monitor_core_bot?start=activate_monitoring
                                    </code>
                                    <button onclick="copyDeepLink()"
                                            class="copy-btn ml-2 px-3 py-1 bg-gray-700 text-white rounded text-sm hover:bg-gray-600">
                                        📋 Copy
                                    </button>
                                </div>
                            </div>

                            <div class="mt-4 flex space-x-3">
                                <a href="https://t.me/my_monitor_core_bot?start=activate_monitoring"
                                   target="_blank"
                                   class="flex-1 inline-flex items-center justify-center px-4 py-2 bg-telegram text-white rounded-lg hover:opacity-90">
                                    <svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
                                        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm4.64 6.8c-.15 1.58-.8 5.42-1.13 7.19-.14.75-.42 1-.68 1.03-.58.05-1.02-.38-1.58-.75-.88-.58-1.38-.94-2.23-1.5-.99-.65-.35-1.01.22-1.59.15-.15 2.71-2.48 2.76-2.69.01-.03.01-.14-.06-.2-.07-.06-.17-.04-.24-.02-.1.02-1.69 1.09-4.78 3.2-.45.31-.86.46-1.23.45-.41-.01-1.2-.23-1.79-.42-.72-.23-1.3-.36-1.25-.76.02-.18.27-.36.75-.55 2.89-1.26 4.83-2.1 5.82-2.51 2.78-1.16 3.36-1.36 3.73-1.36.08 0 .27.02.39.12.1.08.13.19.14.27-.01.06.01.24 0 .38z"/>
                                    </svg>
                                    Buka Deep Link
                                </a>

                                <button onclick="sendDeepLinkToUser()"
                                        class="flex-1 px-4 py-2 border border-green-600 text-green-600 rounded-lg hover:bg-green-50">
                                    Send to User
                                </button>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Configuration Form -->
                <div class="mb-8 p-6 bg-gradient-to-r from-slate-50 to-gray-50 rounded-xl border border-slate-200">
                    <div class="flex items-center mb-6">
                        <div class="w-10 h-10 bg-purple-500 rounded-full flex items-center justify-center mr-4">
                            <span class="text-white font-bold text-lg">3</span>
                        </div>
                        <div>
                            <h3 class="font-bold text-slate-800 text-lg">ENTER CHAT ID & TEST</h3>
                            <p class="text-slate-700">Final configuration and testing</p>
                        </div>
                    </div>

                    <form id="telegram-form" class="space-y-6">
                        <!-- Current Config Status -->
                        <div class="bg-white p-4 rounded-lg border border-slate-200">
                            <h4 class="font-bold text-slate-800 mb-2">Current Configuration Status</h4>
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <p class="text-sm text-slate-600">Chat ID:</p>
                                    <p class="font-mono font-medium text-slate-800" id="current-chat-id">"##);

    html.push_str(if default_chat_id.is_empty() { "Belum diatur" } else { &default_chat_id });

    html.push_str(r##"</p>
                                </div>
                                <div>
                                    <p class="text-sm text-slate-600">Status:</p>
                                    <p class="font-medium " id="current-status">"##);

    html.push_str(if default_enabled { "✅ Aktif" } else { "❌ Nonaktif" });

    html.push_str(r##"</p>
                                </div>
                            </div>
                        </div>

                        <div>
                            <label class="block text-slate-700 font-medium mb-2">
                                <span class="text-red-500">*</span> Your Chat ID
                            </label>
                            <input type="text" id="chat-id" name="chat_id"
                                   class="w-full px-4 py-3 rounded-lg border border-slate-300 focus:border-blue-500 focus:ring-2 focus:ring-blue-200"
                                   placeholder="Enter your Chat ID"
                                   value=""
                                   value=""##);

    html.push_str(&default_chat_id);
    html.push_str(r##""
                                   required>
                            <p class="text-sm text-slate-500 mt-1">Enter Chat ID from @userinfobot</p>
                        </div>

                        <div class="flex items-center">
                            <input type="checkbox" id="enable-telegram" name="enabled"
                                   class="w-5 h-5 text-blue-600 rounded" "##);

    if default_enabled {
        html.push_str("checked");
    }

    html.push_str(r##">
                            <label for="enable-telegram" class="ml-3 text-slate-700 font-medium">
                                Enable Telegram notifications
                            </label>
                        </div>

                        <div class="flex space-x-3 pt-4">
                            <button type="button" onclick="testTelegramConfig()"
                                    class="px-6 py-3 border border-blue-600 text-blue-600 rounded-lg hover:bg-blue-50 flex items-center font-medium">
                                <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                                </svg>
                                Test Configuration
                            </button>
                            <button type="button" onclick="saveTelegramConfig()"
                                    class="px-6 py-3 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700 flex items-center">
                                <svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                                </svg>
                                Save Configuration
                            </button>
                        </div>

                        <div id="result-message" class="mt-4"></div>
                    </form>
                </div>

                <!-- Troubleshooting -->
                <div class="mt-8 p-6 bg-gradient-to-r from-orange-50 to-red-50 rounded-xl border border-orange-200">
                    <h3 class="font-bold text-orange-800 mb-4">🔧 Troubleshooting</h3>
                    <div class="space-y-4">
                        <div class="bg-white p-4 rounded-lg border border-orange-100">
                            <h4 class="font-bold text-orange-800 mb-2">Test failed with "chat not found" error?</h4>
                            <ul class="list-disc pl-5 space-y-1 text-orange-700">
                                <li>Make sure you've sent <code>/start</code> to @my_monitor_core_bot</li>
                                <li>After <code>/start</code>, send a regular message (hello/test)</li>
                                <li>Make sure you haven't blocked the bot</li>
                                <li>Use Deep Link for automatic activation</li>
                            </ul>
                        </div>

                        <div class="bg-white p-4 rounded-lg border border-blue-100">
                            <h4 class="font-bold text-blue-800 mb-2">💡 Tips & Tricks</h4>
                            <ul class="list-disc pl-5 space-y-1 text-blue-700">
                                <li>For group monitoring: use group Chat ID (negative, example: -1001234567890)</li>
                                <li>Personal Chat IDs are always positive</li>
                                <li>Notifications only for websites you monitor</li>
                                <li>Other users cannot see your monitors</li>
                            </ul>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</div>
"##);

    html.push_str(r##"
    <script>
    // Fungsi untuk update status display
    function updateStatusDisplay() {
        const chatId = document.getElementById('chat-id').value;
        const enabled = document.getElementById('enable-telegram').checked;

        document.getElementById('current-chat-id').textContent = chatId || 'Not set';
        document.getElementById('current-status').textContent = enabled ? '✅ Active' : '❌ Inactive';
    }

    function copyDeepLink() {
        const link = 'https://t.me/my_monitor_core_bot?start=activate_monitoring';
        const btn = event.target;

        navigator.clipboard.writeText(link).then(() => {
            // Visual feedback
            const originalText = btn.innerHTML;
            btn.innerHTML = '✓ Copied!';
            btn.classList.add('copied');

            setTimeout(() => {
                btn.innerHTML = originalText;
                btn.classList.remove('copied');
            }, 2000);

            Swal.fire({
                icon: 'success',
                title: 'Link Copied!',
                text: 'Deep link has been copied to clipboard',
                timer: 1500,
                showConfirmButton: false
            });
        });
    }



    function sendDeepLinkToUser() {
        const chatId = document.getElementById('chat-id').value.trim();
        const deepLink = 'https://t.me/my_monitor_core_bot?start=activate_monitoring';

        if (!chatId) {
            Swal.fire({
                icon: 'error',
                title: 'Chat ID Empty',
                text: 'Please enter user Chat ID first',
            });
            return;
        }

        Swal.fire({
            title: 'Kirim Instruksi ke User',
            html: `<div class="text-left">
                  <p>Bot will send complete instructions to:</p>
                  <div class="bg-gray-100 p-3 rounded my-3">
                    <strong>Chat ID:</strong> <code class="bg-gray-200 px-2 py-1 rounded">${chatId}</code>
                  </div>
                  <p>User will receive:</p>
                  <ul class="list-disc pl-5 mt-2 space-y-1">
                    <li>1-click activation link</li>
                    <li>Instructions to get Chat ID</li>
                    <li>How to test configuration</li>
                  </ul>
                  </div>`,
            icon: 'info',
            showCancelButton: true,
            confirmButtonText: 'Ya, Kirim Sekarang',
            cancelButtonText: 'Batal',
            confirmButtonColor: '#3085d6',
            cancelButtonColor: '#d33',
            width: '500px'
        }).then((result) => {
            if (result.isConfirmed) {
                // Show loading
                Swal.fire({
                    title: 'Mengirim...',
                    text: 'Mengirim instruksi ke user',
                    allowOutsideClick: false,
                    didOpen: () => {
                        Swal.showLoading();
                    }
                });

                // Kirim deep link via bot ke user
                fetch('/telegram-setup/send-deeplink', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        chat_id: chatId,
                        deeplink: deepLink
                    })
                })
                .then(response => response.json())
                .then(data => {
                    Swal.close();
                    if (data.success) {
                        Swal.fire({
                            icon: 'success',
                            title: 'Success!',
                            text: 'Instructions have been sent to user via Telegram',
                            timer: 2000,
                            showConfirmButton: false
                        });
                    } else {
                        Swal.fire({
                            icon: 'error',
                            title: 'Failed',
                            html: `<div class="text-left">
                                  <p>${data.message}</p>
                                  <p class="mt-2 text-sm">Make sure user has sent /start to bot first.</p>
                                  </div>`,
                        });
                    }
                })
                .catch(error => {
                    Swal.close();
                    console.error('Error:', error);
                    Swal.fire({
                        icon: 'error',
                        title: 'Network Error',
                        text: 'Tidak bisa connect ke server',
                    });
                });
            }
        });
    }

    function testTelegramConfig() {
        const chatId = document.getElementById('chat-id').value.trim();
        const enabled = document.getElementById('enable-telegram').checked;

        if (!chatId) {
            Swal.fire({
                icon: 'error',
                title: 'Chat ID Empty',
                text: 'Please enter Chat ID from @userinfobot first',
            });
            return;
        }

        // Show loading
        const originalText = event.target.innerHTML;
        event.target.innerHTML = '<div class="animate-spin rounded-full h-5 w-5 border-t-2 border-b-2 border-blue-500 mr-2"></div> Testing...';
        event.target.disabled = true;

        // Kirim hanya chat_id dan enabled (bot_token tidak dikirim)
        fetch('/telegram-setup/test', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                chat_id: chatId,
                enabled: enabled
            })
        })
        .then(response => response.json())
        .then(data => {
            if (data.success) {
                Swal.fire({
                    icon: 'success',
                    title: 'Test Successful!',
                    html: `✅ Telegram notification test successful<br><br>
                          <div class="text-left bg-green-50 p-4 rounded">
                            <div class="grid grid-cols-2 gap-3">
                                <div>
                                    <p class="text-sm text-gray-600">Chat ID:</p>
                                    <p class="font-mono font-bold">${chatId}</p>
                                </div>
                                <div>
                                    <p class="text-sm text-gray-600">Status:</p>
                                    <p class="font-bold text-green-600">Active</p>
                                </div>
                            </div>
                            <p class="mt-3 text-sm text-gray-700">Check your Telegram for test message</p>
                          </div>`,
                    width: '500px'
                });
            } else {
                // Parse error message dengan line breaks
                const errorHtml = data.message.replace(/\n/g, '<br>');
                Swal.fire({
                    icon: 'error',
                    title: 'Test Failed',
                    html: `<div class="text-left">
                          <p class="mb-3">${errorHtml}</p>
                          <div class="mt-4 p-3 bg-yellow-50 rounded border border-yellow-200">
                            <p class="text-sm text-yellow-800">
                                <strong>Solution:</strong><br>
                                1. Open Telegram<br>
                                2. Search @my_monitor_core_bot<br>
                                3. Send: /start<br>
                                4. Send: hello<br>
                                5. Try test again
                            </p>
                          </div>
                          </div>`,
                    width: '600px'
                });
            }
        })
        .catch(error => {
            console.error('Error:', error);
            Swal.fire({
                icon: 'error',
                title: 'Network Error',
                text: 'Tidak bisa connect ke server: ' + error.message,
            });
        })
        .finally(() => {
            // Restore button
            event.target.innerHTML = originalText;
            event.target.disabled = false;
        });
    }

    function saveTelegramConfig() {
        const chatId = document.getElementById('chat-id').value.trim();
        const enabled = document.getElementById('enable-telegram').checked;

        if (!chatId) {
            Swal.fire({
                icon: 'error',
                title: 'Chat ID Empty',
                text: 'Please enter your Telegram Chat ID',
            });
            return;
        }

        // Show loading
        const originalText = event.target.innerHTML;
        event.target.innerHTML = '<div class="animate-spin rounded-full h-5 w-5 border-t-2 border-b-2 border-blue-500 mr-2"></div> Saving...';
        event.target.disabled = true;

        // Kirim hanya chat_id dan enabled (bot_token TIDAK dikirim)
        fetch('/telegram-setup/save', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                chat_id: chatId,
                enabled: enabled
            })
        })
        .then(response => response.json())
        .then(data => {
            if (data.success) {
                Swal.fire({
                    icon: 'success',
                    title: 'Configuration Saved!',
                    html: `✅ Telegram notifications have been enabled<br><br>
                          <div class="text-left bg-blue-50 p-4 rounded">
                            <p class="font-medium">You will now receive:</p>
                            <ul class="list-disc pl-5 mt-2 space-y-1">
                                <li>Alert when website is DOWN</li>
                                <li>Alert when website is UP (recovery)</li>
                                <li>Real-time notifications</li>
                            </ul>
                          </div>`,
                    width: '500px'
                }).then(() => {
                    updateStatusDisplay();
                });
            } else {
                Swal.fire({
                    icon: 'error',
                    title: 'Failed to Save',
                    text: data.message,
                });
            }
        })
        .catch(error => {
            console.error('Error:', error);
            Swal.fire({
                icon: 'error',
                title: 'Error',
                text: 'Network error: ' + error.message,
            });
        })
        .finally(() => {
            // Restore button
            event.target.innerHTML = originalText;
            event.target.disabled = false;
        });
    }

    // Update status display saat halaman load
    document.addEventListener('DOMContentLoaded', function() {
        updateStatusDisplay();

        // Update status saat input berubah
        document.getElementById('chat-id').addEventListener('input', updateStatusDisplay);
        document.getElementById('enable-telegram').addEventListener('change', updateStatusDisplay);

        // Auto-fill with example Chat ID if empty
        const chatIdInput = document.getElementById('chat-id');
        if (!chatIdInput.value) {
            chatIdInput.placeholder = 'Enter your Chat ID';
        }
    });
    </script>
"##);

    html.push_str(&footer::render());
    html.push_str("</body></html>");

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

#[derive(serde::Deserialize)]
struct TelegramConfigForm {
    chat_id: String,
    enabled: bool,
}

#[derive(serde::Deserialize)]
struct DeeplinkRequest {
    chat_id: String,
    deeplink: String,
}

async fn save_telegram_config(
    session: Session,
    db_pool: web::Data<SqlitePool>,
    form: web::Json<TelegramConfigForm>,
) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            println!("💾 Saving Telegram config for user_id: {}, chat_id: {}, enabled: {}",
                     user_id, form.chat_id, form.enabled);

            match TelegramNotifier::save_config(
                &db_pool,
                user_id,
                &form.chat_id,
                form.enabled,
            ).await {
                Ok(_) => {
                    println!("✅ Telegram config saved successfully");
                    HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "message": "Telegram notifications configured successfully!"
                    }))
                }
                Err(e) => {
                    println!("❌ Failed to save Telegram config: {}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "success": false,
                        "message": format!("Database error: {}", e)
                    }))
                }
            }
        }
        _ => {
            println!("❌ Unauthorized save attempt");
            HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "message": "Unauthorized"
            }))
        }
    }
}

async fn test_telegram_config(
    session: Session,
    form: web::Json<TelegramConfigForm>,
) -> impl Responder {
    use crate::telegram::TelegramConfig;

    // Log untuk debugging
    println!("📱 Testing Telegram config:");
    println!("   Chat ID: {}", form.chat_id);
    println!("   Enabled: {}", form.enabled);

    // Check session
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            println!("   User ID: {}", user_id);
        }
        _ => {
            println!("   No user session found");
        }
    }

    // Ambil bot token dari environment variable
    let global_token = std::env::var("TELEGRAM_GLOBAL_BOT_TOKEN")
        .unwrap_or_else(|_| {
            let default_token = "8572929961:AAGg52v7JK9v5SHrkHKmheX9eV6EBmo8uRQ".to_string();
            println!("⚠️ Using default token");
            default_token
        });

    println!("   Bot token length: {}", global_token.len());

    let config = TelegramConfig {
        bot_token: global_token,
        chat_id: form.chat_id.clone(),
        enabled: true,
    };

    println!("   🚀 Attempting to send Telegram message...");

    match TelegramNotifier::send_message(
        &config,
        "✅ <b>Test Alert dari MonitorCore</b>\n\nNotifikasi Telegram Anda berhasil dikonfigurasi!\n\nAnda akan menerima alert saat website yang Anda monitor:\n• Status DOWN (dari UP)\n• Status UP (recovery dari DOWN)\n\n<code>Test completed successfully</code>"
    ).await {
        Ok(_) => {
            println!("   ✅ Test message sent successfully to {}", form.chat_id);
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Test message sent successfully"
            }))
        }
        Err(e) => {
            let error_msg = e.to_string();
            println!("   ❌ Failed to send test message: {}", error_msg);

            // Deteksi error spesifik
            let user_message = if error_msg.contains("chat not found") {
                format!("❌ User belum memulai chat dengan bot!\n\nLangkah PERBAIKAN:\n1. Buka Telegram di HP/PC\n2. Cari '@my_monitor_core_bot'\n3. Kirim: /start\n4. Kirim pesan lain: hello\n5. Coba test lagi\n\nChat ID Anda: {}", form.chat_id)
            } else if error_msg.contains("bot was blocked") {
                format!("❌ Bot diblokir oleh user. Silahkan unblock @my_monitor_core_bot\nChat ID: {}", form.chat_id)
            } else if error_msg.contains("403") {
                format!("❌ Bot tidak bisa mengirim pesan ke user\nChat ID: {}\n\nPastikan user sudah mengirim /start ke @my_monitor_core_bot", form.chat_id)
            } else {
                format!("❌ Error: {}", error_msg)
            };

            HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "message": user_message
            }))
        }
    }
}

async fn send_deeplink(
    session: Session,
    form: web::Json<DeeplinkRequest>,
) -> impl Responder {
    use crate::telegram::TelegramConfig;

    // Check session
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => {
            println!("📤 User {} sending deeplink to chat_id: {}", user_id, form.chat_id);
        }
        _ => {
            println!("❌ Unauthorized deeplink attempt");
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "message": "Unauthorized"
            }));
        }
    }

    let global_token = std::env::var("TELEGRAM_GLOBAL_BOT_TOKEN")
        .unwrap_or_else(|_| "8572929961:AAGg52v7JK9v5SHrkHKmheX9eV6EBmo8uRQ".to_string());

    let config = TelegramConfig {
        bot_token: global_token,
        chat_id: form.chat_id.clone(),
        enabled: true,
    };

    // Kirim pesan instruksi lengkap
    let message = format!(
        "🔧 <b>Setup MonitorCore Notifications</b>\n\n\
        <b>Langkah 1:</b> Aktifkan bot dengan klik link ini:\n\
        <code>{}</code>\n\n\
        <b>Langkah 2:</b> Setelah bot aktif, kembali ke website dan:\n\
        1. Masukkan Chat ID Anda\n\
        2. Klik \"Test Configuration\"\n\
        3. Klik \"Save Configuration\"\n\n\
        <b>Cara mendapatkan Chat ID:</b>\n\
        1. Buka @userinfobot di Telegram\n\
        2. Kirim /start\n\
        3. Salin angka \"Id:\" yang diberikan\n\n\
        <i>Jika sudah memiliki Chat ID, abaikan langkah di atas.</i>",
        form.deeplink
    );

    match TelegramNotifier::send_message(&config, &message).await {
        Ok(_) => {
            println!("✅ Deep link sent to {}", form.chat_id);
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Deep link sent successfully"
            }))
        }
        Err(e) => {
            println!("❌ Failed to send deep link: {}", e);
            HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to send deep link: {}", e)
            }))
        }
    }
}
