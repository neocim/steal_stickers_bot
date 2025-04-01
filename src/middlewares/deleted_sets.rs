use chrono::{NaiveTime, Utc};
use std::{ops::DerefMut, time::Duration};
use tokio::sync::{Mutex, RwLock};
use tracing::debug;

use telers::{
    errors::{session::ErrorKind, EventErrorKind, MiddlewareError, TelegramErrorKind},
    event::EventReturn,
    methods::GetStickerSet,
    middlewares::{outer::MiddlewareResponse, OuterMiddleware},
    Bot, Request,
};

use async_trait::async_trait;

use crate::application::{
    commands::set_deleted_col::set_deleted_col,
    common::traits::uow::UoW as UoWTrait,
    set::{
        dto::{get_by_tg_id::GetByTgID, set_deleted_col_by_short_name::SetDeletedColByShortName},
        traits::SetRepo,
    },
};

#[derive(Debug)]
pub struct DeletedSetsMiddleware<UoW> {
    bot: Mutex<Bot>,
    uow: RwLock<UoW>,
    last_update_time: Mutex<NaiveTime>,
}

impl<UoW> DeletedSetsMiddleware<UoW>
where
    UoW: UoWTrait,
{
    pub fn new(uow: UoW, bot: Bot) -> Self {
        Self {
            uow: RwLock::new(uow),
            last_update_time: Mutex::new(Utc::now().time()),
            bot: Mutex::new(bot),
        }
    }
}

#[async_trait]
impl<UoW> OuterMiddleware for DeletedSetsMiddleware<UoW>
where
    UoW: UoWTrait + Send + Sync,
    for<'a> UoW::SetRepo<'a>: Send + Sync,
{
    async fn call(&self, request: Request) -> Result<MiddlewareResponse, EventErrorKind> {
        let mut uow = self.uow.write().await;

        let mut last_upd_time_lock = self.last_update_time.lock().await;
        let now = Utc::now().time();

        let bot = self.bot.lock().await;

        if (now - *last_upd_time_lock).num_hours() >= 2 {
            *last_upd_time_lock = now;

            let user_id = match request.update.from_id() {
                Some(id) => id,
                None => {
                    return Ok((request, EventReturn::Skip));
                }
            };

            debug!(user_id, "Update database deleted sticker sets by user id:");

            let uow = uow.deref_mut();

            let sets = uow
                .set_repo()
                .await
                .map_err(MiddlewareError::new)?
                .get_by_tg_id(GetByTgID::new(user_id, Some(false)))
                .await
                .map_err(MiddlewareError::new)?;

            for (i, sticker) in sets.into_iter().enumerate() {
                if let Err(err) = bot
                    .send(GetStickerSet::new(sticker.short_name.as_str()))
                    .await
                {
                    if matches!(err,  ErrorKind::Telegram(TelegramErrorKind::BadRequest { message }) if message.as_ref()
                    == "Bad Request: STICKERSET_INVALID")
                    {
                        set_deleted_col(
                            uow,
                            SetDeletedColByShortName::new(sticker.short_name.as_str(), true),
                        )
                        .await
                        .map_err(MiddlewareError::new)?;
                    }
                }

                if i % 5 == 0 {
                    tokio::time::sleep(Duration::from_millis(1010)).await;
                }
            }
        }
        Ok((request, EventReturn::Skip))
    }
}
