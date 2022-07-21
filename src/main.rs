//! main.rs
use env_logger::Env;
use sqlx::PgPool;
use std::net::TcpListener;
use zero2prod::settings::get_config;
use zero2prod::startup::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let config = get_config().expect("Failed to read config");
    let pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to postgres");
    let address = format!("127.0.0.1:{}", config.port);
    let listener = TcpListener::bind(address).expect("Failed to bind port");

    run(listener, pool)?.await
}
