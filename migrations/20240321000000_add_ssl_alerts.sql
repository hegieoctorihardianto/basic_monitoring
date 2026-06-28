-- Add ssl_alerts table for SSL certificate monitoring
CREATE TABLE IF NOT EXISTS ssl_alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    ssl_monitor_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    severity TEXT NOT NULL CHECK(severity IN ('critical', 'warning', 'info')),
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'acknowledged', 'resolved')),
    triggered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    resolved_at TIMESTAMP,
    acknowledged_at TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (ssl_monitor_id) REFERENCES ssl_monitors(id) ON DELETE CASCADE
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_ssl_alerts_user_id ON ssl_alerts(user_id);
CREATE INDEX IF NOT EXISTS idx_ssl_alerts_ssl_monitor_id ON ssl_alerts(ssl_monitor_id);
CREATE INDEX IF NOT EXISTS idx_ssl_alerts_status ON ssl_alerts(status);
CREATE INDEX IF NOT EXISTS idx_ssl_alerts_triggered ON ssl_alerts(triggered_at);
CREATE INDEX IF NOT EXISTS idx_ssl_alerts_severity ON ssl_alerts(severity);
