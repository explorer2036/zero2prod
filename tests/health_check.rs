//! tests/health_check.rs
use once_cell::sync::Lazy;
use reqwest::header;
use secrecy::ExposeSecret;
use sha3::Digest;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::email_client::EmailClient;
use zero2prod::settings::{get_config, DatabaseSettings};
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

pub struct TestApp {
    pub address: String,
    pub pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
}

pub struct TestUser {
    pub id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let password = format!("{:x}", sha3::Sha3_256::digest(self.password.as_bytes()));
        sqlx::query!(
            "INSERT INTO users (id, username, password) VALUES ($1, $2, $3)",
            self.id,
            self.username,
            password,
        )
        .execute(pool)
        .await
        .expect("Failed to create test user");
    }
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", &self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn test_user(&self) -> (String, String) {
        let row = sqlx::query!("SELECT username, password FROM users LIMIT 1")
            .fetch_one(&self.pool)
            .await
            .expect("Failed to create test  users");
        (row.username, row.password)
    }
}

// Lauch our applicationin the background
async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;
    let mut config = get_config().expect("Failed to read config");
    config.database.db_name = Uuid::new_v4().to_string();
    config.email_client.base_url = email_server.uri();
    let pool = configurate_database(&config.database).await;

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

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind address");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let server = run(listener, pool.clone(), email_client).expect("Failed to new server");
    // launch the server as a background task
    // tokio::spawn returns a handle to the spawned future
    let _ = tokio::spawn(server);

    let test_app = TestApp {
        address,
        pool,
        email_server,
        test_user: TestUser::generate(),
    };
    test_app.test_user.store(&test_app.pool).await;

    test_app
}

async fn configurate_database(config: &DatabaseSettings) -> PgPool {
    // create database
    let mut connection =
        PgConnection::connect(&config.connection_string_without_db().expose_secret())
            .await
            .expect("Failed to connect to postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.db_name).as_str())
        .await
        .expect("Failed to create database");

    // migrate database
    let pool = PgPool::connect(&config.connection_string().expose_secret())
        .await
        .expect("Failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate the database");

    pool
}

// tokio::test is the testing equivalent of tokio::main
// It also spares you from having to specify the `#[test]` attribute.
//
// You can inspect what code gets generated using
// `cargo expand --test health_check` (<- name of the test file)
#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app = spawn_app().await;

    let body = "name=alon&email=alonlong%40gmail.com";
    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.pool.clone())
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "alonlong@gmail.com");
    assert_eq!(saved.name, "alon");
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let app = spawn_app().await;

    let body = "name=alon&email=alonlong%40gmail.com";
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN token;")
        .execute(&app.pool)
        .await
        .unwrap();

    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(500, response.status().as_u16());
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    let app = spawn_app().await;

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
               "text": "Newsletter body as plain text",
               "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()[header::WWW_AUTHENTICATE]
    );
}
