use telers::{
    Bot,
    errors::{TelegramErrorKind, session::ErrorKind},
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::{GetStickerSet, SendMessage},
    types::{MessageText, ReplyParameters},
};

use tracing::error;

use crate::{
    core::helpers::{
        constants::{TELEGRAM_SET_SHORT_NAME_MAX_LENGTH, TELEGRAM_STICKER_SET_URL},
        texts::{invalid_set, now_enter_set_name},
    },
    presentation::commands::{
        common::send_detailed_error_msg, states::steal_sticker_set::StealStickerSetState,
    },
};

pub async fn process_sticker_pack_links<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    let msg = message.text.as_ref();

    if let Some(link_idx) = msg.find(TELEGRAM_STICKER_SET_URL) {
        let short_name_start = link_idx + TELEGRAM_STICKER_SET_URL.len();

        let short_name: String = msg
            .chars()
            .skip(short_name_start)
            .take(TELEGRAM_SET_SHORT_NAME_MAX_LENGTH)
            .take_while(|c| c.is_ascii_alphabetic() || *c == '_')
            .collect();

        if let Err(ref error) = bot.send(GetStickerSet::new(&*short_name)).await {
            if matches!(error, ErrorKind::Telegram(TelegramErrorKind::BadRequest { message }) if **message == *"Bad Request: STICKERSET_INVALID")
            {
                bot.send(
                    SendMessage::new(message.chat.id(), invalid_set()).reply_parameters(
                        ReplyParameters::new()
                            .chat_id(message.chat.id())
                            .message_id(message.message_id),
                    ),
                )
                .await?;

                return Ok(EventReturn::Finish);
            } else {
                fsm.set_value("steal_sticker_set_name", short_name)
                    .await
                    .map_err(Into::into)?;

                fsm.set_state(StealStickerSetState::CreateNewStickerSet)
                    .await
                    .map_err(Into::into)?;

                bot.send(
                    SendMessage::new(message.chat.id(), now_enter_set_name()).reply_parameters(
                        ReplyParameters::new()
                            .chat_id(message.chat.id())
                            .message_id(message.message_id),
                    ),
                )
                .await?;
            }

            error!(
                ?error,
                "Error occurred while getting sticker set name to steal: "
            );

            send_detailed_error_msg(&bot, &error.to_string(), message.chat.id()).await?;

            return Ok(EventReturn::Finish);
        }
    }

    Ok(EventReturn::Finish)
}
