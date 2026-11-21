use telers::{
    Bot,
    enums::ParseMode,
    errors::{TelegramErrorKind, session::ErrorKind},
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::{AnswerCallbackQuery, EditMessageText, SendMessage},
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message},
};

use crate::core::helpers::texts::{help_message, start_message};

pub async fn start_handler<S: Storage>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    let buttons = [[InlineKeyboardButton::new("Help").callback_data("help")]];
    let keyboard = InlineKeyboardMarkup::new(buttons);

    bot.send(
        SendMessage::new(
            message.chat().id(),
            start_message(&message.from().expect("error while parsing user").first_name),
        )
        .reply_markup(keyboard),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn help_callback_handler(bot: Bot, callback_query: CallbackQuery) -> HandlerResult {
    bot.send(AnswerCallbackQuery::new(callback_query.id))
        .await?;

    let (chat_id, message_id) = if let Some(message) = callback_query.message {
        (message.chat().id(), message.message_id())
    } else {
        return Ok(EventReturn::Finish);
    };

    let edit_message = EditMessageText::new()
        .text(help_message())
        .chat_id(chat_id)
        .message_id(message_id)
        .parse_mode(ParseMode::HTML);

    if let Err(error) = bot.send(edit_message.parse_mode(ParseMode::HTML)).await {
        match &error {
            ErrorKind::Telegram(TelegramErrorKind::BadRequest { message }) => {
                // we need to ignore this bad request error
                if !message.contains("message is not modified") {
                    return Err(error.into());
                }
            }
            _ => return Err(error.into()),
        }
    }

    Ok(EventReturn::Finish)
}

pub async fn help_handler<S: Storage>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    bot.send(SendMessage::new(message.chat().id(), help_message()))
        .await?;

    Ok(EventReturn::Finish)
}
