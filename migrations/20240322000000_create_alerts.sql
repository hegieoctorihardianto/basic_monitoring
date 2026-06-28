-- Create alerts table for website monitoring
CREATE TABLE IF NOT EXISTS alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    monitor_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    severity TEXT NOT NULL CHECK(severity IN ('critical', 'warning', 'info')),
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'acknowledged', 'resolved')),
    triggered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    resolved_at TIMESTAMP,
    acknowledged_at TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (monitor_id) REFERENCES monitors(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_alerts_user_id ON alerts(user_id);
CREATE INDEX IF NOT EXISTS idx_alerts_monitor_id ON alerts(monitor_id);
CREATE INDEX IF NOT EXISTS idx_alerts_status ON alerts(status);
CREATE INDEX IF NOT EXISTS idx_alerts_triggered ON alerts(triggered_at);
CREATE INDEX IF NOT EXISTS idx_alerts_severity ON alerts(severity);
