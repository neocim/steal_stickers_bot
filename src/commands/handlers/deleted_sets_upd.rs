use sqlx::{Database, Pool};
use telers::{Bot, event::simple::HandlerResult};

use crate::application::{
    common::traits::uow::UoWFactory as UoWFactoryTrait, set::traits::SetRepo,
    user::traits::UserRepo,
};
use crate::infrastructure::database::{
    repositories::{set::SetRepoImpl, user::UserRepoImpl},
    uow::UoWFactory,
};

pub async fn deleted_sets_upd<DB>(pool: Pool<DB>, bot: Bot) -> HandlerResult
where
    DB: Database,
    for<'a> UserRepoImpl<&'a mut DB::Connection>: UserRepo,
    for<'a> SetRepoImpl<&'a mut DB::Connection>: SetRepo,
{
    tokio::spawn(async move {
        let uow_factory = UoWFactory::new(pool);

        loop {
            let f = uow_factory.create_uow();
        }
    });

    Ok(())
}
