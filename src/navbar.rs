pub fn render(email: &str) -> String {
    format!(r#"
    <!-- Top Navigation -->
    <nav class="bg-white shadow">
        <div class="max-w-7xl mx-auto px-4">
            <div class="flex justify-between h-16">
                <div class="flex items-center">
                    <div class="flex items-center space-x-3">
                        <div class="w-10 h-10 bg-slate-800 rounded-lg flex items-center justify-center">
                            <span class="text-xl text-white">📊</span>
                        </div>
                        <span class="text-xl font-bold text-slate-800">BasicMonitoring</span>
                    </div>
                </div>
                <div class="flex items-center space-x-6">
                    <span class="text-slate-600">Welcome, <span class="font-semibold">{}</span></span>
                    <a href="/dashboard" class="text-slate-600 hover:text-slate-900">Dashboard</a>
                    <a href="/monitors" class="text-slate-600 hover:text-slate-900">Monitors</a>
                    <a href="/alerts" class="text-slate-600 hover:text-slate-900">Alerts</a>
                    <a href="/logout" class="text-red-600 hover:text-red-800">Logout</a>
                </div>
            </div>
        </div>
    </nav>
    "#, email)
}