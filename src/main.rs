use std::process;

use application::common::traits::uow::UoWFactory as _;
use infrastructure::database::uow::UoWFactory;
use sqlx::Postgres;
use telers::{
    client::Reqwest,
    enums::ContentType as ContentTypeEnum,
    errors::HandlerError,
    event::ToServiceProvider as _,
    fsm::{MemoryStorage, Strategy},
    methods::SetMyCommands,
    middlewares::outer::FSMContext,
    types::{BotCommand, BotCommandScopeAllPrivateChats},
    Bot, Dispatcher, Router,
};

use clap::{Parser, Subcommand};
use tracing::{debug, error};
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

pub mod application;
pub mod bot_commands;
pub mod config;
pub mod core;
pub mod domain;
pub mod infrastructure;
pub mod middlewares;
mod telegram_application;

use bot_commands::{
    add_stickers_command, cancel_command, my_stickers, process_non_command, process_non_sticker,
    source_command, start_command, steal_sticker_set_command,
};
use config::ConfigToml;
use core::texts;
use middlewares::{
    ClientApplicationMiddleware, CreateUserMiddleware, DatabaseMiddleware, DeletedSetsMiddleware,
};
use telegram_application::{client_authorize, client_connect};

async fn set_commands(bot: Bot) -> Result<(), HandlerError> {
    let help = BotCommand::new("help", "Show help message");
    let source = BotCommand::new("source", "Show the source of the bot");
    let src = BotCommand::new("src", "Show the source of the bot");
    let steal = BotCommand::new("stealpack", "Steal sticker pack");
    let steal_sticker = BotCommand::new(
        "addstickers",
        "Add stickers to a sticker pack stolen by this bot",
    );
    let my_stickers = BotCommand::new("mystickers", "List of your stolen stickers");
    let cancel = BotCommand::new("cancel", "Cancel last command");

    let private_chats = [help, source, src, steal, steal_sticker, cancel, my_stickers];

    bot.send(SetMyCommands::new(private_chats).scope(BotCommandScopeAllPrivateChats {}))
        .await?;

    Ok(())
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, PartialEq)]
enum Commands {
    /// Authorize client and exit
    Auth,
    /// Run programm (exit if client not authorized)
    Run,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let config = std::fs::read_to_string("configs/config.toml")
        .expect("error occurded while read config file");
    let config: ConfigToml = toml::from_str(&config).unwrap();

    let log_level = match std::env::var("LOG_LEVEL") {
        Ok(log_level) => log_level,
        Err(_) => config.clone().tracing.log_level,
    };

    let db_url = config.get_postgres_url();

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

    let (api_id, api_hash) = (config.tg_app.api_id, config.tg_app.api_hash);

    debug!("Connecting client..");

    let client = match client_connect(api_id, api_hash.clone()).await {
        Ok(client) => client,
        Err(err) => {
            error!(?err, "An error occurded while client connect:");

            process::exit(1);
        }
    };

    debug!("Client connected");

    let cli = Cli::parse();

    if Commands::Auth == cli.command {
        if let Err(err) = client_authorize(
            &client,
            config.auth.phone_number.as_str(),
            config.auth.password.as_str(),
        )
        .await
        {
            error!(?err, "An error occurded while client authorize:");

            process::exit(1);
        };

        debug!("Client sucessfully authorized! Now run programm using command:\njust compose-run OR just compose-run-build");

        process::exit(0);
    }
    if Commands::Run == cli.command && !client.is_authorized().await.expect("error to authorize") {
        error!("Client is not authorized! Run programm with command auth:\njust auth");

        process::exit(1);
    }

    debug!("Connecting to the database..");

    let pool = match sqlx::PgPool::connect(&db_url).await {
        Ok(pool) => pool,
        Err(err) => {
            error!(?err, "An error occurded while connect to database:");

            process::exit(1);
        }
    };

    debug!("Connected to database");

    let bot = Bot::new(config.bot.bot_token);

    let mut main_router: Router<Reqwest> = Router::new("main");
    let mut private_router = Router::new("private");

    let storage = MemoryStorage::new();

    private_router
        .update
        .outer_middlewares
        .register(FSMContext::new(storage).strategy(Strategy::UserInChat));

    private_router
        .update
        .outer_middlewares
        .register(DatabaseMiddleware::new(UoWFactory::new(pool.clone())));

    private_router
        .update
        .outer_middlewares
        .register(ClientApplicationMiddleware::new(client, api_id, api_hash));

    private_router
        .update
        .outer_middlewares
        .register(CreateUserMiddleware::new(
            UoWFactory::new(pool.clone()).create_uow(),
        ));

    private_router
        .update
        .outer_middlewares
        .register(DeletedSetsMiddleware::new(
            UoWFactory::new(pool).create_uow(),
            bot.clone(),
        ));

    process_non_command(
        &mut private_router,
        &[
            "source",
            "src",
            "stealpack",
            "addstickers",
            "help",
            "cancel",
            "mystickers",
        ],
    )
    .await;

    start_command(&mut private_router, &["start", "help"]).await;

    source_command(&mut private_router, &["src", "source"]).await;

    cancel_command(&mut private_router, &["cancel"]).await;

    add_stickers_command::<Postgres>(&mut private_router, "addstickers", "done").await;

    steal_sticker_set_command::<Postgres>(&mut private_router, "stealpack").await;

    my_stickers::<Postgres>(&mut private_router, "mystickers").await;

    process_non_sticker(&mut private_router, ContentTypeEnum::Sticker).await;

    main_router.include(private_router);
    main_router.startup.register(set_commands, (bot.clone(),));

    let dispatcher = Dispatcher::builder()
        .bot(bot)
        .allowed_updates(main_router.resolve_used_update_types())
        .router(main_router)
        .build();

    match dispatcher
        .to_service_provider_default()
        .expect("error occurded while convert the service factory to the service")
        .run_polling()
        .await
    {
        Ok(()) => debug!("Bot stopped"),
        Err(err) => debug!("Bot stopped with error: {err}"),
    }
}
