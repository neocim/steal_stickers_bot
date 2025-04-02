use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{NaiveTime, Utc};
use telers::{
    Bot, Request,
    errors::{EventErrorKind, MiddlewareError, TelegramErrorKind, session::ErrorKind},
    event::EventReturn,
    methods::GetStickerSet,
    middlewares::{OuterMiddleware, outer::MiddlewareResponse},
};
use tracing::debug;

use crate::application::{
    commands::set_deleted_col::set_deleted_col,
    common::traits::uow::UoW as UoWTrait,
    set::{
        dto::{get_by_tg_id::GetByTgID, set_deleted_col_by_short_name::SetDeletedColByShortName},
        traits::SetRepo,
    },
};

#[derive(Debug, Clone)]
pub struct DeletedSetsMiddleware<UoW> {
    bot: Arc<Bot>,
    uow: UoW,
    last_update_time: Arc<NaiveTime>,
}

impl<UoW> DeletedSetsMiddleware<UoW>
where
    UoW: UoWTrait,
{
    pub fn new(uow: UoW, bot: Bot) -> Self {
        Self {
            uow: uow,
            last_update_time: Arc::new(Utc::now().time()),
            bot: Arc::new(bot),
        }
    }
}

#[async_trait]
impl<UoW> OuterMiddleware for DeletedSetsMiddleware<UoW>
where
    UoW: UoWTrait + Send + Sync + Clone + 'static,
    for<'a> UoW::SetRepo<'a>: Send + Sync,
{
    async fn call(&mut self, request: Request) -> Result<MiddlewareResponse, EventErrorKind> {
        let now = Utc::now().time();

        if (now - *self.last_update_time).num_hours() >= 2 {
            self.last_update_time = Arc::new(now);

            let user_id = match request.update.from_id() {
                Some(id) => id,
                None => {
                    return Ok((request, EventReturn::Skip));
                }
            };

            debug!(user_id, "Update database deleted sticker sets by user id:");
            let sets = self
                .uow
                .set_repo()
                .await
                .map_err(MiddlewareError::new)?
                .get_by_tg_id(GetByTgID::new(user_id, Some(false)))
                .await
                .map_err(MiddlewareError::new)?;

            for (i, sticker) in sets.into_iter().enumerate() {
                if let Err(err) = &self
                    .bot
                    .send(GetStickerSet::new(sticker.short_name.as_str()))
                    .await
                {
                    if matches!(err,  ErrorKind::Telegram(TelegramErrorKind::BadRequest { message }) if message.as_ref()
                    == "Bad Request: STICKERSET_INVALID")
                    {
                        set_deleted_col(
                            &mut self.uow,
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
