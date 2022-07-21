//! main.rs
use sqlx::{Connection, PgConnection};
use std::net::TcpListener;
use zero2prod::settings::get_config;
use zero2prod::startup::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = get_config().expect("Failed to read config");
    let connection = PgConnection::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to postgres");
    let address = format!("127.0.0.1:{}", config.port);
    let listener = TcpListener::bind(address).expect("Failed to bind port");

    run(listener, connection)?.await
}
