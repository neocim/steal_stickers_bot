use std::process;

use clap::{Parser, Subcommand};
use grammers_client::Client;
use sqlx::{Pool, Postgres};
use telers::Bot;
use tracing::{error, info};

pub mod application;
mod config;
mod core;
mod domain;
mod infrastructure;
mod presentation;

use crate::{
    config::{get_config_toml, init_tracing_subscriber_from_config},
    presentation::{client_authorize, client_connect, start_bot},
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, PartialEq)]
pub enum Commands {
    /// Authorize client and exit
    Auth,
    /// Run programm (exit if client not authorized)
    Start,
    /// Apply database migrations and exit
    Migrate,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    let config = get_config_toml();
    init_tracing_subscriber_from_config(&config);

    let pg_url = config.get_postgres_url();

    match cli.command {
        Commands::Migrate => {
            let pool = connect_database(&pg_url).await;
            apply_migrations(&pool).await;
            process::exit(0);
        }
        Commands::Auth => {
            let client = connect_client(config.tg_app.api_id).await;
            authorize_client(
                &client,
                &config.tg_app.api_hash,
                &config.auth.phone_number,
                &config.auth.password,
            )
            .await;
            process::exit(0);
        }
        Commands::Start => {
            run_bot(&config, &pg_url).await;
        }
    }
}

async fn run_bot(config: &crate::config::ConfigToml, pg_url: &str) {
    let bot = Bot::new(&config.bot.bot_token);

    let client = connect_client(config.tg_app.api_id).await;
    ensure_client_authorized(&client).await;

    let pool = connect_database(pg_url).await;

    start_bot(bot, pool, client).await;
}

async fn connect_client(api_id: i32) -> Client {
    info!("Connecting client..");
    match client_connect(api_id).await {
        Ok(client) => {
            info!("Client connected!");
            client
        }
        Err(err) => {
            error!(?err, "An error occurred while client connecting: ");
            process::exit(1);
        }
    }
}

async fn authorize_client(client: &Client, api_hash: &str, phone: &str, password: &str) {
    info!("Trying to log in..");
    if let Err(err) = client_authorize(client, api_hash, phone, password).await {
        error!(?err, "An error occurred while client authorize:");
        process::exit(1);
    }
    info!("Client sucessfully authorized!");
}

async fn ensure_client_authorized(client: &Client) {
    if !client.is_authorized().await.expect("error to authorize") {
        error!("Client is not authorized!");
        process::exit(1);
    }
}

async fn connect_database(pg_url: &str) -> Pool<Postgres> {
    info!("Connecting to the database..");
    match sqlx::PgPool::connect(pg_url).await {
        Ok(pool) => {
            info!("Connected the database!");
            pool
        }
        Err(err) => {
            error!(?err, "An error occurred while connect to database:");
            process::exit(1);
        }
    }
}

async fn apply_migrations(pool: &Pool<Postgres>) {
    info!("Applying database migrations..");
    match sqlx::migrate!("./src/infrastructure/database/migrations")
        .run(pool)
        .await
    {
        Ok(()) => info!("Migrations applied successfully!"),
        Err(err) => {
            error!(?err, "An error occurred while applying migrations:");
            process::exit(1);
        }
    }
}
