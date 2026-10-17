use crate::{
    core::helpers::{
        constants::TELEGRAM_STICKER_SET_URL,
        texts::{do_you_want_to_steal, now_enter_set_name, start_message, sticker_without_pack},
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
    methods::SendMessage,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Message, ReplyParameters, Sticker},
};

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

        return Ok(EventReturn::Finish);
    };

    let set_name: String = fsm
        .get_value("steal_sticker_set_name")
        .await
        .map_err(Into::into)?
        .expect("Sticker set name for sticker set user want steal should be set");

    let buttons = vec![vec![
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

    ///
    fsm.set_state(StealStickerSetState::CreateNewStickerSet)
        .await
        .map_err(Into::into)?;

    Ok(EventReturn::Finish)
}
