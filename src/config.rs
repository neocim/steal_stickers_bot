use serde::Deserialize;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _};

pub fn init_tracing_subscriber_from_config(config: &ConfigToml) {
    // If we specify env `LOG_LEVEL`, use it value, config value otherwise
    let log_level = match std::env::var("LOG_LEVEL") {
        Ok(log_level) => log_level,
        Err(_) => config.clone().tracing.log_level,
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::new(log_level)
                .add_directive("hyper=warn".parse().expect("Invalid directive"))
                .add_directive("reqwest=warn".parse().expect("Invalid directive"))
                .add_directive("grammers=warn".parse().expect("Invalid directive"))
                .add_directive("sqlx=warn".parse().expect("Invalid directive"))
                .add_directive(
                    "telers::client::session::base=off"
                        .parse()
                        .expect("Invalid directive"),
                ),
        )
        .init();
}

pub fn get_postgres_url(config: &ConfigToml) -> String {
    config.get_postgres_url()
}

pub fn get_config_toml() -> ConfigToml {
    let config = std::fs::read_to_string("configs/config.toml")
        .expect("error occurded while reading config file");

    toml::from_str(&config).unwrap()
}

impl ConfigToml {
    pub fn get_postgres_url(&self) -> String {
        let postgres = &self.postgres;

        format!(
            "postgres://{}:{}@{}:{}/{}",
            postgres.username, postgres.password, postgres.host, postgres.port, postgres.db
        )
    }
}

#[derive(Deserialize, Clone)]
pub struct ConfigToml {
    pub bot: BotConfig,
    pub tg_app: Application,
    pub auth: AuthCredentials,
    pub tracing: Tracing,
    pub postgres: DatabaseConfig,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: String,
    pub db: String,
}

#[derive(Deserialize, Clone)]
pub struct BotConfig {
    pub bot_token: String,
}

#[derive(Deserialize, Clone)]
pub struct Application {
    pub api_id: i32,
    pub api_hash: String,
}

#[derive(Deserialize, Clone)]
pub struct AuthCredentials {
    pub phone_number: String,
    pub password: String,
}

#[derive(Deserialize, Clone)]
pub struct Tracing {
    pub log_level: String,
}
