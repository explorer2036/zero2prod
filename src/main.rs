//! main.rs
use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::net::TcpListener;
use zero2prod::email_client::EmailClient;
use zero2prod::settings::get_config;
use zero2prod::startup::run;
use zero2prod::telemetry::get_subscriber;
use zero2prod::telemetry::init_subscriber;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = get_config().expect("Failed to read config");
    let pool = PgPool::connect(&config.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to postgres");

    let sender_email = config
        .email_client
        .sender()
        .expect("Invalid sender email address");
    let email_client = EmailClient::new(
        config.email_client.base_url.clone(),
        sender_email,
        config.email_client.token.clone(),
        config.email_client.timeout().clone(),
    );

    let address = format!("{}:{}", config.application.host, config.application.port);
    log::info!("server address: {}", address);
    let listener = TcpListener::bind(address)?;
    run(listener, pool, email_client)?.await?;
    Ok(())
}
