use crate::{
    core::helpers::{
        constants::TELEGRAM_STICKER_SET_URL,
        texts::{
            do_you_want_to_steal, now_enter_set_name, start_message, you_can_steal_at_any_time,
        },
    },
    presentation::commands::{
        common::{process_link_to_steal, process_sticker_to_steal},
        states::{callback_data::StealSetAction, steal_sticker_set::StealStickerSetState},
    },
};
use telers::{
    Bot,
    callback_data::CallbackData as _,
    enums::ParseMode,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::{AnswerCallbackQuery, EditMessageText, SendMessage},
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message, ReplyParameters},
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
    } else {
        bot.send(SendMessage::new(
            message.chat().id(),
            // only can panic if messages uses in channels, but i'm using private filter in main function
            start_message(&message.from().expect("error while parsing user").first_name),
        ))
        .await?;

        return Ok(EventReturn::Finish);
    };

    let set_name: String = fsm
        .get_value("steal_sticker_set_name")
        .await
        .map_err(Into::into)?
        .expect("Sticker set name for sticker set user want steal should be set");

    let yes_data = StealSetAction {
        action: "Yes".into(),
    }
    .pack()
    .expect("Failed to pack StealSetAction::Yes");
    let no_data = StealSetAction {
        action: "No".into(),
    }
    .pack()
    .expect("Failed to pack StealSetAction::No");

    let buttons = [[
        InlineKeyboardButton::new("Yes").callback_data(yes_data),
        InlineKeyboardButton::new("No").callback_data(no_data),
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
    StealSetAction { action }: StealSetAction,
    fsm: Context<S>,
) -> HandlerResult {
    let (chat_id, message_id) = if let Some(message) = callback_query.message {
        (message.chat().id(), message.message_id())
    } else {
        return Ok(EventReturn::Finish);
    };

    if action == "Yes" {
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
