-- Add ssl_monitors table
CREATE TABLE IF NOT EXISTS ssl_monitors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    domain TEXT NOT NULL,
    port INTEGER DEFAULT 443,
    check_interval INTEGER DEFAULT 24,
    alert_days TEXT DEFAULT '30,7,1',
    last_checked TIMESTAMP,
    last_status TEXT,
    expiry_date TIMESTAMP,
    issuer TEXT,
    is_active BOOLEAN DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ssl_monitors_user_id ON ssl_monitors(user_id);
CREATE INDEX IF NOT EXISTS idx_ssl_monitors_expiry ON ssl_monitors(expiry_date);
CREATE INDEX IF NOT EXISTS idx_ssl_monitors_status ON ssl_monitors(last_status);
