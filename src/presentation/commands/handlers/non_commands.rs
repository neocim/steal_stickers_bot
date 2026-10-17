use crate::{
    core::helpers::{
        constants::TELEGRAM_STICKER_SET_URL,
        texts::{
            do_you_want_to_steal, now_enter_set_name, start_message, sticker_without_pack,
            you_can_steal_at_any_time,
        },
    },
    presentation::commands::{
        common::{
            InvalidSetLinkError, check_set_name, parse_sticker_set_link, process_link_to_steal,
            process_sticker_to_steal, send_detailed_error_msg, send_invalid_set_msg,
        },
        states::{callback_data::CallbackDataPrefix, steal_sticker_set::StealStickerSetState},
    },
};
use telers::{
    Bot,
    enums::ParseMode,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::{AnswerCallbackQuery, EditMessageText, SendMessage},
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message, ReplyParameters},
};
use tracing::debug;

pub async fn process_non_commands_handler<S: Storage>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    if let Some(text) = message.text()
        && text.contains(TELEGRAM_STICKER_SET_URL)
    {
        process_link_to_steal(&bot, message.chat().id(), message.message_id(), &fsm, text).await?;
    } else if let Some(sticker) = message.sticker() {
        process_sticker_to_steal(
            &bot,
            message.chat().id(),
            message.message_id(),
            &fsm,
            sticker,
        )
        .await?;
    };

    let set_name: String = fsm
        .get_value("steal_sticker_set_name")
        .await
        .map_err(Into::into)?
        .expect("Sticker set name for sticker set user want steal should be set");

    let buttons = [[
        InlineKeyboardButton::new("Yes").callback_data(format!(
            "{prefix}Yes",
            prefix = CallbackDataPrefix::StealSet.as_str()
        )),
        InlineKeyboardButton::new("No").callback_data(format!(
            "{prefix}No",
            prefix = CallbackDataPrefix::StealSet.as_str()
        )),
    ]];

    let inline_keyboard_markup = InlineKeyboardMarkup::new(buttons);

    bot.send(
        SendMessage::new(message.chat().id(), do_you_want_to_steal(&set_name))
            .parse_mode(ParseMode::HTML)
            .reply_markup(inline_keyboard_markup)
            .reply_parameters(
                ReplyParameters::new()
                    .chat_id(message.chat().id())
                    .message_id(message.message_id()),
            ),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn process_user_agreement_buttons<S: Storage>(
    bot: Bot,
    callback_query: CallbackQuery,
    fsm: Context<S>,
) -> HandlerResult {
    debug!("111111111111111111111111");
    let (chat_id, message_id) = if let Some(message) = callback_query.message {
        if let Some(chat_id) = message.chat_id() {
            (chat_id, message.message_id())
        } else {
            return Ok(EventReturn::Finish);
        }
    } else {
        return Ok(EventReturn::Finish);
    };
    debug!("{}", callback_query.data.as_ref().unwrap());

    if callback_query.data.unwrap().ends_with("Yes") {
        bot.send(
            EditMessageText::new()
                .text(now_enter_set_name())
                .chat_id(chat_id)
                .message_id(message_id),
        )
        .await?;

        fsm.set_state(StealStickerSetState::CreateNewStickerSet)
            .await
            .map_err(Into::into)?;
    } else {
        bot.send(
            EditMessageText::new()
                .text(you_can_steal_at_any_time())
                .chat_id(chat_id)
                .message_id(message_id),
        )
        .await?;
    }

    bot.send(AnswerCallbackQuery::new(callback_query.id))
        .await?;

    return Ok(EventReturn::Finish);
}
