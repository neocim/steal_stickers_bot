use grammers_client::Client;
use telers::{
    Bot, Extension,
    enums::ParseMode,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::SendMessage,
    types::{MessageSticker, MessageText, ReplyParameters},
    utils::text::html_code,
};
use tracing::error;

use crate::presentation::{
    commands::{common::send_default_error_message, states::get_owner::GetOwnerState},
    telegram_application::get_sticker_set_user_id,
};

pub async fn get_owner_handler<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        "Send me a sticker and i'll show you the owner of this sticker pack.",
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
    let set_name = message.sticker.set_name.unwrap();

    let owner_id = match get_sticker_set_user_id(&set_name, &client).await {
        Ok(id) => id,
        Err(error) => {
            error!(
                ?error,
                ?set_name,
                "Error occurred while getting sticker set user id: "
            );

            send_default_error_message(&bot, message.chat.id()).await?;

            return Ok(EventReturn::Finish);
        }
    };

    bot.send(
        SendMessage::new(
            message.chat.id(),
            format!(
                "ID the owner of this sticker pack — {}. Send the next one or use /cancel instead.",
                html_code(owner_id.to_string())
            ),
        )
        .parse_mode(ParseMode::HTML)
        .reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())),
    )
    .await?;

    Ok(EventReturn::Finish)
}
