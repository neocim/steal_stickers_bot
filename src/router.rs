use grammers_client::Client;
use sqlx::{Pool, Postgres};
use telers::{
    Bot, Dispatcher, Router, enums,
    filters::ChatType,
    fsm::{MemoryStorage, Strategy},
    middlewares::outer::FSMContext,
};
use tracing::debug;

use crate::{
    commands::{deleted_sets_upd, init_commands, set_commands},
    infrastructure::database::uow::UoWFactory,
    middlewares::CreateUserMiddleware,
};

pub async fn start_bot(bot: &'static Bot, pool: Pool<Postgres>, client: Client) {
    let mut router = init_router(bot, pool.clone());

    let dispatcher = Dispatcher::builder()
        .main_router(router.clone().configure_default())
        .bot(bot.clone())
        .allowed_updates(router.resolve_used_update_types())
        .extension(client)
        .extension(UoWFactory::new(pool))
        .build();

    match dispatcher.run_polling().await {
        Ok(()) => debug!("Bot stopped"),
        Err(err) => debug!("Bot stopped with error: {err}"),
    }
}

fn init_router(bot: &'static Bot, pool: Pool<Postgres>) -> Router {
    let mut main_router = Router::new("main");
    let mut private_router = Router::new("private");

    init_commands(&mut private_router);

    private_router
        .update
        .filter(ChatType::one(enums::ChatType::Private));

    private_router
        .update
        .outer_middlewares
        .register(FSMContext::new(MemoryStorage::new()).strategy(Strategy::UserInChat));

    private_router
        .update
        .outer_middlewares
        .register(CreateUserMiddleware::new(UoWFactory::new(pool.clone())));

    main_router
        .startup
        .register(deleted_sets_upd, (pool.clone(), bot.clone()));

    main_router.include(private_router);
    main_router.startup.register(set_commands, (bot,));

    main_router
}
