use telers::{
    Bot, Extension,
    enums::ParseMode,
    errors::HandlerError,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context as FSMContext, Storage},
    methods::{AnswerCallbackQuery, SendMessage},
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message, ReplyMarkup},
};

use crate::{
    application::{
        common::traits::uow::{UoW, UoWFactory as UoWFactoryTrait},
        set::{dto::count_by_tg_id::CountByTgID, repository::SetRepo as _},
        user::repository::UserRepo as _,
    },
    core::texts::personal_stats_message,
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

    let not_deleted_user_sets_count = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .count_by_tg_id(CountByTgID::new(user_id, Some(false)))
        .await
        .map_err(HandlerError::new)?;

    let all_user_sets_count = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .count_by_tg_id(CountByTgID::new(user_id, None))
        .await
        .map_err(HandlerError::new)?;

    let buttons = vec![vec![
        InlineKeyboardButton::new("Global Stats »").callback_data(format!(
            "{prefix}Global",
            prefix = CallbackDataPrefix::Stats.as_str()
        )),
    ]];

    let reply_markup = ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(buttons));

    bot.send(
        SendMessage::new(
            message.chat().id(),
            personal_stats_message(all_user_sets_count, not_deleted_user_sets_count),
        )
        .parse_mode(ParseMode::HTML)
        .reply_markup(reply_markup),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn process_buttons<S, UoWFactory>(
    bot: Bot,
    callback_query: CallbackQuery,
    fsm: FSMContext<S>,
    Extension(uow_factory): Extension<UoWFactory>,
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

    bot.send(AnswerCallbackQuery::new(callback_query.id))
        .await?;

    Ok(EventReturn::Finish)
}
