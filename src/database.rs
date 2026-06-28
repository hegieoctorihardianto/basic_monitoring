use sqlx::{SqlitePool, migrate::MigrateDatabase};
use std::env;
use dotenv::dotenv;

pub async fn init() -> SqlitePool {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:data/monitoring.db".to_string());

    // Buat folder data jika belum ada
    if let Some(path) = database_url.strip_prefix("sqlite:") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    // Buat database jika belum ada
    if !sqlx::Sqlite::database_exists(&database_url).await.unwrap_or(false) {
        println!("📦 Creating database...");
        sqlx::Sqlite::create_database(&database_url).await.unwrap();
    }

    // Connect ke database
    let pool = SqlitePool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    match sqlx::migrate!("./migrations").run(&pool).await {
        Ok(_) => println!("✅ Database migrations applied"),
        Err(e) => eprintln!("❌ Migration error: {}", e),
    }

    pool
}

pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Self {
        let pool = init().await;
        Self { pool }
    }
}
