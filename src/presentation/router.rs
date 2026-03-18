use grammers_client::Client;
use sqlx::{Pool, Postgres};
use telers::{
    Bot, Dispatcher, Router, enums,
    event::simple,
    filters::ChatType,
    fsm::{MemoryStorage, Strategy},
    middlewares::outer::FSMContext,
};
use tracing::debug;

use crate::{
    infrastructure::database::uow::UoWFactory,
    presentation::commands::{PrivateRouterBuilder, deleted_sets_upd, set_commands},
};

pub async fn start_bot(bot: Bot, pool: Pool<Postgres>, client: Client) {
    let router = init_router(bot.clone(), pool.clone());

    let dispatcher = Dispatcher::builder()
        .main_router(router.clone().configure_default())
        .bot(bot)
        .allowed_updates(router.resolve_used_update_types())
        .extension(client)
        .extension(UoWFactory::new(pool))
        .build();

    match dispatcher.run_polling().await {
        Ok(()) => debug!("Bot stopped"),
        Err(err) => debug!("Bot stopped with error: {err}"),
    }
}

fn init_router(bot: Bot, pool: Pool<Postgres>) -> Router {
    let private_router = PrivateRouterBuilder::init(Router::new("private"), pool.clone());
    let main_router = Router::new("main")
        .include(private_router)
        .on_startup(|observer| {
            observer
                .register(simple::Handler::new(set_commands, (bot.clone(),)))
                .register(simple::Handler::new(deleted_sets_upd, (pool, bot.clone())))
        });

    main_router
}
