use crate::{
    core::helpers::{
        constants::TELEGRAM_STICKER_SET_URL,
        texts::{now_enter_set_name, start_message},
    },
    presentation::commands::{
        common::{
            InvalidSetLinkError, parse_sticker_set_link, send_detailed_error_msg,
            send_invalid_set_msg,
        },
        states::steal_sticker_set::StealStickerSetState,
    },
};
use telers::{
    Bot,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::SendMessage,
    types::{Message, ReplyParameters},
};

pub async fn process_non_commands_handler<S: Storage>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    let text = if let Some(text) = message.text()
        && text.contains(TELEGRAM_STICKER_SET_URL)
    {
        process_link(&bot, message.chat().id(), message.message_id(), fsm, text)
    } else {
        bot.send(SendMessage::new(
            message.message_id(),
            // only can panic if messages uses in channels, but i'm using private filter in main function
            start_message(&message.from().expect("error while parsing user").first_name),
        ))
        .await?;

        return Ok(EventReturn::Finish);
    };

    Ok(EventReturn::Finish)
}

async fn process_link<S: Storage>(
    bot: &Bot,
    chat_id: i64,
    message_id: i64,
    fsm: Context<S>,
    text: &str,
) -> HandlerResult {
    let set_name = match parse_sticker_set_link(&bot, text).await {
        Ok(set_name) => set_name,
        Err(err) => match err.downcast_ref() {
            Some(InvalidSetLinkError) => {
                return send_invalid_set_msg(&bot, chat_id, message_id).await;
            }
            None => {
                send_detailed_error_msg(&bot, &err.to_string(), chat_id, message_id).await?;
                return Ok(EventReturn::Finish);
            }
        },
    };

    fsm.set_value("steal_sticker_set_name", set_name)
        .await
        .map_err(Into::into)?;

    fsm.set_state(StealStickerSetState::CreateNewStickerSet)
        .await
        .map_err(Into::into)?;

    bot.send(
        SendMessage::new(chat_id, now_enter_set_name()).reply_parameters(
            ReplyParameters::new()
                .chat_id(chat_id)
                .message_id(message_id),
        ),
    )
    .await?;

    Ok(EventReturn::Finish)
}
