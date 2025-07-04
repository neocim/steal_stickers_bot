use std::process;

use clap::{Parser, Subcommand};
use grammers_client::Client;
use telers::Bot;
use tracing::{debug, error};

pub mod application;
mod config;
mod core;
mod domain;
mod infrastructure;
mod presentation;

use crate::{
    config::{get_config_toml, init_tracing_subscriber_from_config},
    presentation::{
        router::start_bot,
        telegram_application::{client_authorize, client_connect},
    },
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let config = get_config_toml();
    let pg_url = config.get_postgres_url();
    // FIXME!: perhaps there is another, more profitable way to create a variable that lives the entire program.
    let bot = Box::leak(Box::new(Bot::new(&config.bot.bot_token)));
    let (api_id, api_hash) = (config.tg_app.api_id, config.tg_app.api_hash.clone());
    init_tracing_subscriber_from_config(&config);

    debug!("Connecting client..");
    let client = match client_connect(api_id, api_hash.clone()).await {
        Ok(client) => client,
        Err(err) => {
            error!(?err, "An error occurred while client connecting: ");

            process::exit(1);
        }
    };
    debug!("Client connected!");

    debug!("Trying to log in..");
    run_or_auth(&client, &config.auth.phone_number, &config.auth.password).await;
    debug!("Successfully logged in!");

    debug!("Connecting to the database with url `{pg_url}`..");
    let pool = match sqlx::PgPool::connect(&pg_url).await {
        Ok(pool) => pool,
        Err(err) => {
            error!(?err, "An error occurred while connect to database:");

            process::exit(1);
        }
    };
    debug!("Connected the database!");

    start_bot(bot, pool, client).await;
}

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
    Run,
}

async fn run_or_auth(client: &Client, ph_num: &str, pswd: &str) {
    let cli = Cli::parse();

    if Commands::Auth == cli.command {
        if let Err(err) = client_authorize(client, ph_num, pswd).await {
            error!(?err, "An error occurred while client authorize:");

            process::exit(1);
        };
        debug!(
            "Client sucessfully authorized! Now run programm using command:\njust compose-run OR just compose-build"
        );

        process::exit(0);
    }
    if Commands::Run == cli.command && !client.is_authorized().await.expect("error to authorize") {
        error!("Client is not authorized! Run programm with command auth:\njust auth");

        process::exit(1);
    }
}
