use std::borrow::Cow;

use telers::{
    Bot, Extension,
    errors::HandlerError,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::SendMessage,
    types::Message,
};

use crate::application::{
    common::traits::uow::{UoW, UoWFactory as UoWFactoryTrait},
    set::{dto::count_by_tg_id::CountByTgID, repository::SetRepo},
};

pub async fn stats_handler<S, UoWFactory>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
    Extension(uow_factory): Extension<UoWFactory>,
) -> HandlerResult
where
    S: Storage,
    UoWFactory: UoWFactoryTrait,
{
    fsm.finish().await.map_err(Into::into)?;
    bot.send(SendMessage::new(message.chat().id(), "IN DEVELOPMENT"))
        .await?;

    Ok(EventReturn::Finish)
}

#[derive(Debug, Clone)]
struct GetUserStatsError {
    message: Cow<'static, str>,
}

pub async fn get_user_stats<UoWFactory>(
    bot: Bot,
    message: Message,
    uow_factory: UoWFactory,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
{
    let user_id = match message.from_id() {
        Some(id) => id,
        None => return Ok(EventReturn::Finish),
    };

    let mut uow = uow_factory.create_uow();
    let count = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .count_by_tg_id(CountByTgID::new(user_id, Some(false)))
        .await
        .map_err(HandlerError::new)?;

    Ok(EventReturn::Finish)
}
