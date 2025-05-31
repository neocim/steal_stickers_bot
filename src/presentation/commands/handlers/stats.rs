use telers::{
    Bot, Extension,
    errors::HandlerError,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::SendMessage,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Message, ReplyMarkup},
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

    let mut uow = uow_factory.create_uow();

    let user_id = match message.from_id() {
        Some(id) => id,
        None => return Ok(EventReturn::Finish),
    };

    let count = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .count_by_tg_id(CountByTgID::new(user_id, Some(false)))
        .await
        .map_err(HandlerError::new)?;

    let buttons = vec![vec![
        InlineKeyboardButton::new("Global stats").callback_data(""),
    ]];

    let reply_markup = ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(buttons));

    bot.send(
        SendMessage::new(message.chat().id(), format!("HELLO {count}")).reply_markup(reply_markup),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn process_buttons<UoWFactory>(
    bot: Bot,
    user_id: i64,
    chat_id: i64,
    uow_factory: UoWFactory,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
{
    let mut uow = uow_factory.create_uow();
    let count = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .count_by_tg_id(CountByTgID::new(user_id, Some(false)))
        .await
        .map_err(HandlerError::new)?;

    bot.send(SendMessage::new(chat_id, format!("HELLO {count}")))
        .await?;

    Ok(EventReturn::Finish)
}
