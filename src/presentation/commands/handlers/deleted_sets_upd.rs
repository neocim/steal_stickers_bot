#![allow(unused_must_use)]

use chrono::{Duration, Utc};
use sqlx::{Database, Pool};
use telers::errors::{SessionErrorKind, TelegramErrorKind};
use telers::methods::GetStickerSet;
use telers::{Bot, event::simple::HandlerResult};
use tracing::{debug, error};

use crate::application::common::traits::uow::UoW as _;
use crate::application::interactors::set_deleted_col::set_deleted_col;
use crate::application::set::dto::get_all::GetAll;
use crate::application::set::dto::set_deleted_col_by_short_name::SetDeletedColByShortName;
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
        let uow_factory = UoWFactory::new(pool.clone());
        let mut last_upd_time = Utc::now();

        debug!("Start checking for deleted sets.");

        loop {
            if Utc::now() - last_upd_time < Duration::hours(12) {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                continue;
            }

            debug!(
                "Start changing the `deleted` columns for sticker sets that have already been deleted. Current time: `{:?}`",
                Utc::now()
            );

            let mut uow = uow_factory.create_uow();
            let result = uow.set_repo().await;

            if let Err(err) = result {
                error!("Failed to start transaction: {:?}", err);
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }

            let result = result.unwrap().get_all(GetAll::new(Some(false))).await;
            if let Err(err) = result {
                error!("Error occurded while trying to get all sets: {:?}", err);
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }

            let sets = result.unwrap();
            for (i, set) in sets.into_iter().enumerate() {
                if let Err(err) = bot.send(GetStickerSet::new(set.short_name.as_str())).await {
                    if matches!(err,  SessionErrorKind::Telegram(TelegramErrorKind::BadRequest { message })
                        if message.as_ref() == "Bad Request: STICKERSET_INVALID")
                    {
                        debug!(
                            "Trying to set `deleted` column to `true` for sticker set `{}`..",
                            set.short_name.as_str()
                        );
                        set_deleted_col(
                            &mut uow,
                            SetDeletedColByShortName::new(set.short_name.as_str(), true),
                        )
                        .await
                        .map_err(|err| {
                            error!(
                                "Failed to update `deleted` column for sticker set {}: {:?}",
                                set.short_name, err
                            );
                        });
                    }
                }
                if i % 5 == 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1010)).await;
                }
            }
            last_upd_time = Utc::now();
            debug!(
                "Finish changing the `deleted` columns. Current time: `{:?}`",
                last_upd_time
            );
        }
    });

    Ok(())
}
