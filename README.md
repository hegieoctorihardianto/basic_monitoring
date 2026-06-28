# 🚀 Rust Monitoring Server

**Production-ready website & SSL certificate monitoring tool built with Rust.**

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Commercial-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20FreeBSD%20%7C%20Windows-lightgrey?style=flat-square)]()

---

## ✨ Features

- 🌐 **Website Monitoring** - Monitor website uptime & performance
- 🔒 **SSL Certificate Monitoring** - Track SSL expiry dates
- 📧 **Email Notifications** - Get alerts via email
- 🤖 **Telegram Notifications** - Real-time alerts to Telegram
- 📊 **Interactive Dashboard** - Clean UI with real-time updates
- 📈 **Performance Reports** - Export to CSV/JSON/HTML
- 🏷️ **Public Status Page** - Share status with clients
- 🔔 **Alert System** - Critical/Warning/Info alerts
- 🗄️ **SQLite Database** - Lightweight & portable

---

## 🚀 Quick Start

### Prerequisites
- [Rust](https://rustup.rs/) (1.70+)
- Git (optional)

### Installation

```bash
# Clone repository
git clone git@github.com:hegieoctorihardianto/basic_monitoring.git
cd basic_monitoring

# Copy environment config
cp .env.example .env

# Edit .env with your settings
nano .env

# Build
cargo build --release

# Run
./target/release/rust_monitoring_server