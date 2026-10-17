use telers::{
    Bot,
    errors::{TelegramErrorKind, session::ErrorKind},
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::{GetStickerSet, SendMessage},
    types::{Message, ReplyParameters},
};

use tracing::{debug, error};

use crate::{
    core::helpers::{
        constants::{TELEGRAM_SET_SHORT_NAME_MAX_LENGTH, TELEGRAM_STICKER_SET_URL},
        texts::{invalid_set, now_enter_set_name, start_message},
    },
    presentation::commands::{
        common::send_detailed_error_msg, states::steal_sticker_set::StealStickerSetState,
    },
};

pub async fn process_non_commands_handler<S: Storage>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    let msg = if let Some(text) = message.text() {
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

    if let Some(link_idx) = msg.find(TELEGRAM_STICKER_SET_URL) {
        let short_name_start = link_idx + TELEGRAM_STICKER_SET_URL.len();

        let short_name: String = msg
            .chars()
            .skip(short_name_start)
            .take(TELEGRAM_SET_SHORT_NAME_MAX_LENGTH)
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();

        debug!(short_name);

        if short_name.len() < 1 {
            bot.send(
                SendMessage::new(message.chat().id(), invalid_set()).reply_parameters(
                    ReplyParameters::new()
                        .chat_id(message.chat().id())
                        .message_id(message.message_id()),
                ),
            )
            .await?;
        }

        if let Err(ref error) = bot.send(GetStickerSet::new(&*short_name)).await {
            if matches!(error, ErrorKind::Telegram(TelegramErrorKind::BadRequest { message }) if **message == *"Bad Request: STICKERSET_INVALID")
            {
                debug!("1");
                bot.send(
                    SendMessage::new(message.chat().id(), invalid_set()).reply_parameters(
                        ReplyParameters::new()
                            .chat_id(message.chat().id())
                            .message_id(message.message_id()),
                    ),
                )
                .await?;

                return Ok(EventReturn::Finish);
            }

            error!(
                ?error,
                "Error occurred while getting sticker set name to steal: "
            );

            send_detailed_error_msg(&bot, &error.to_string(), message.chat().id()).await?;

            return Ok(EventReturn::Finish);
        }

        debug!("2");
        fsm.set_value("steal_sticker_set_name", short_name)
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
    }

    Ok(EventReturn::Skip)
}
