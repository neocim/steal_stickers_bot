use telers::{
    Bot,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::SendMessage,
    types::{Message, ReplyParameters},
};

use crate::{
    core::helpers::{
        constants::{TELEGRAM_SET_SHORT_NAME_MAX_LENGTH, TELEGRAM_STICKER_SET_URL},
        texts::{invalid_set, now_enter_set_name, start_message},
    },
    presentation::commands::{
        common::{check_set_name, parse_sticker_set_link},
        states::steal_sticker_set::StealStickerSetState,
    },
};

pub async fn process_non_commands_handler<S: Storage>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    let text = if let Some(text) = message.text() {
        text
    } else {
        bot.send(SendMessage::new(
            message.chat().id(),
            // only can panic if messages uses in channels, but i'm using private filter in main function
            start_message(&message.from().expect("error while parsing user").first_name),
        ))
        .await?;

        return Ok(EventReturn::Finish);
    };

    let set_name = if let Ok(set_name) = parse_sticker_set_link(&bot, &message, text).await {
        set_name
    } else {
        return Ok(EventReturn::Skip);
    };

    fsm.set_value("steal_sticker_set_name", set_name)
        .await
        .map_err(Into::into)?;

    fsm.set_state(StealStickerSetState::CreateNewStickerSet)
        .await
        .map_err(Into::into)?;

    bot.send(
        SendMessage::new(message.chat().id(), now_enter_set_name()).reply_parameters(
            ReplyParameters::new()
                .chat_id(message.chat().id())
                .message_id(message.message_id()),
        ),
    )
    .await?;

    Ok(EventReturn::Finish)
}
