use grammers_client::Client;
use telers::{
    Bot, Extension,
    enums::ParseMode,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::SendMessage,
    types::{MessageSticker, MessageText, ReplyParameters},
};
use tracing::error;

use crate::{
    core::helpers::texts::{
        ask_for_sticker_for_owner, pack_owner_id, sticker_without_pack_for_owner,
    },
    presentation::{
        commands::{common::send_default_error_msg, states::get_owner::GetOwnerState},
        telegram_application::get_sticker_set_user_id,
    },
};

pub async fn get_owner_handler<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        ask_for_sticker_for_owner(),
    ))
    .await?;

    fsm.set_state(GetOwnerState::GetStickers)
        .await
        .map_err(Into::into)?;

    Ok(EventReturn::Finish)
}

pub async fn get_owner_id(
    bot: Bot,
    message: MessageSticker,
    Extension(client): Extension<Client>,
) -> HandlerResult {
    let set_name = match message.sticker.set_name() {
        Some(set_name) => set_name,
        None => {
            bot.send(
                SendMessage::new(message.chat.id(), sticker_without_pack_for_owner())
                    .reply_parameters(
                        ReplyParameters::new()
                            .chat_id(message.chat.id())
                            .message_id(message.message_id),
                    ),
            )
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    let owner_id = match get_sticker_set_user_id(&set_name, &client).await {
        Ok(id) => id,
        Err(error) => {
            error!(
                ?error,
                ?set_name,
                "Error occurred while getting sticker set user id: "
            );

            send_default_error_msg(&bot, message.chat.id(), message.message_id).await?;

            return Ok(EventReturn::Finish);
        }
    };

    bot.send(
        SendMessage::new(message.chat.id(), pack_owner_id(owner_id))
            .parse_mode(ParseMode::HTML)
            .reply_parameters(
                ReplyParameters::new()
                    .chat_id(message.chat.id())
                    .message_id(message.message_id),
            ),
    )
    .await?;

    Ok(EventReturn::Finish)
}
