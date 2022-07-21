//! src/config.rs

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub port: u16,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub user: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub db_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.db_name
        )
    }

    pub fn connection_string_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.user, self.password, self.host, self.port
        )
    }
}

pub fn get_config() -> Result<Settings, config::ConfigError> {
    // init the configuration reader
    let mut settings = config::Config::default();

    // add configuration values from a file named `config`.
    // it will look for any top-level file with an extension
    // that `config` knows how to parse: yaml, json etc
    settings.merge(config::File::with_name("config"))?;

    // try to convert the config values into `Settings` type
    settings.try_into()
}
