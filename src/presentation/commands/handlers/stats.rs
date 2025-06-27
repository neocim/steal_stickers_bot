use telers::{
    Bot, Extension,
    errors::HandlerError,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context as FSMContext, Storage},
    methods::SendMessage,
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message, ReplyMarkup},
};

use crate::{
    application::{
        common::traits::uow::{UoW, UoWFactory as UoWFactoryTrait},
        set::repository::SetRepo,
        user::repository::UserRepo,
    },
    presentation::commands::states::callback_data::CallbackDataPrefix,
};

pub async fn stats_handler<S, UoWFactory>(
    bot: Bot,
    message: Message,
    fsm: FSMContext<S>,
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

    let buttons = vec![vec![
        InlineKeyboardButton::new("Global stats").callback_data(format!(
            "{prefix}Global",
            prefix = CallbackDataPrefix::Stats.as_str()
        )),
    ]];

    let reply_markup = ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(buttons));

    Ok(EventReturn::Finish)
}

pub async fn process_buttons<S, UoWFactory>(
    bot: Bot,
    callback_query: CallbackQuery,
    fsm: FSMContext<S>,
    uow_factory: UoWFactory,
) -> HandlerResult
where
    S: Storage,
    UoWFactory: UoWFactoryTrait,
{
    let mut uow = uow_factory.create_uow();

    let (chat_id, message_id) = match (callback_query.chat_id(), callback_query.message_id()) {
        (Some(chat_id), Some(message_id)) => (chat_id, message_id),
        _ => return Ok(EventReturn::Finish),
    };

    Ok(EventReturn::Finish)
}
