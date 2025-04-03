use grammers_client::Client;
use sqlx::{Pool, Postgres};
use telers::{
    Bot, Dispatcher, Router,
    fsm::{MemoryStorage, Strategy},
    middlewares::outer::FSMContext,
};
use tracing::debug;

use crate::{
    commands::{deleted_sets_upd, init_commands, set_commands},
    infrastructure::database::uow::UoWFactory,
    middlewares::{ClientApplicationMiddleware, CreateUserMiddleware, DatabaseMiddleware},
};

pub async fn start_bot(
    bot: &Bot,
    pool: Pool<Postgres>,
    client: Client,
    api_id: i32,
    api_hash: String,
) {
    let mut router = init_router(bot, pool, client, api_id, api_hash);
    init_commands(&mut router);

    let dispatcher = Dispatcher::builder()
        .main_router(router.clone().configure_default())
        .bot(bot.clone())
        .allowed_updates(router.resolve_used_update_types())
        .build();

    match dispatcher.run_polling().await {
        Ok(()) => debug!("Bot stopped"),
        Err(err) => debug!("Bot stopped with error: {err}"),
    }
}

fn init_router(
    bot: &Bot,
    pool: Pool<Postgres>,
    client: Client,
    api_id: i32,
    api_hash: String,
) -> Router {
    let mut main_router = Router::new("main");
    let mut private_router = Router::new("private");

    private_router
        .update
        .outer_middlewares
        .register(FSMContext::new(MemoryStorage::new()).strategy(Strategy::UserInChat));

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
        .register(CreateUserMiddleware::new(UoWFactory::new(pool.clone())));

    private_router
        .startup
        .register(deleted_sets_upd, (pool.clone(), bot.clone()));

    main_router.include(private_router);
    main_router.startup.register(set_commands, (&bot.clone(),));

    main_router
}
