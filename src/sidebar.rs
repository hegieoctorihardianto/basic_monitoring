pub fn render() -> String {
    r#"
    <!-- Sidebar -->
    <div class="sidebar-bg text-white w-64 min-h-screen p-6 hidden md:block">
        <div class="mb-10">
            <h2 class="text-lg font-semibold mb-4">Monitoring</h2>
            <ul class="space-y-2">
                <li>
                    <a href="/dashboard" class="flex items-center space-x-3 p-3 rounded-lg bg-slate-700/50">
                        <span>📊</span>
                        <span>Dashboard</span>
                    </a>
                </li>
                <li>
                    <a href="/monitors" class="flex items-center space-x-3 p-3 rounded-lg hover:bg-slate-700/50">
                        <span>🌐</span>
                        <span>Website Monitors</span>
                    </a>
                </li>
                <li>
                    <a href="/ssl-monitors" class="flex items-center space-x-3 p-3 rounded-lg hover:bg-slate-700/50">
                        <span>🔒</span>
                        <span>SSL Monitors</span>
                    </a>
                </li>
                <li>
                    <a href="/alerts" class="flex items-center space-x-3 p-3 rounded-lg hover:bg-slate-700/50">
                        <span>🔔</span>
                        <span>Alerts</span>
                    </a>
                </li>
                <li>
                    <a href="/reports" class="flex items-center space-x-3 p-3 rounded-lg hover:bg-slate-700/50">
                        <span>📈</span>
                        <span>Reports</span>
                    </a>
                </li>
            </ul>
        </div>

        <div class="mb-10">
            <h2 class="text-lg font-semibold mb-4">Settings</h2>
            <ul class="space-y-2">
                <li>
                    <a href="/email-preferences" class="flex items-center space-x-3 p-3 rounded-lg hover:bg-slate-700/50">
                        <span>📧</span>
                        <span>Email Notifications</span>
                    </a>
                </li>
                <li>
                    <a href="/telegram-setup" class="flex items-center space-x-3 p-3 rounded-lg hover:bg-slate-700/50">
                        <span>🤖</span>
                        <span>Telegram Notifications</span>
                    </a>
                </li>

                <li>
                    <a href="/profile" class="flex items-center space-x-3 p-3 rounded-lg hover:bg-slate-700/50">
                        <span>👤</span>
                        <span>Profile</span>
                    </a>
                </li>
                <li>
                    <a href="/server" class="flex items-center space-x-3 p-3 rounded-lg hover:bg-slate-700/50">
                        <span>🖥️</span>
                        <span>Server Health</span>
                    </a>
                </li>
            </ul>
        </div>

        <div class="mt-auto pt-6 border-t border-slate-700">
            <div class="p-3 bg-slate-700/30 rounded-lg">
                <p class="text-sm">System Status</p>
                <div class="flex items-center mt-2">
                    <div class="w-3 h-3 bg-green-500 rounded-full mr-2"></div>
                    <span class="text-sm">All systems operational</span>
                </div>
            </div>
        </div>
    </div>
    "#.to_string()
}
