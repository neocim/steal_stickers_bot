use grammers_client::{Client, InvocationError};
use telers::{
    Bot, Extension,
    enums::ParseMode,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::{GetMe, SendMessage},
    types::{Message, ReplyParameters},
};
use tracing::error;

use crate::{
    core::helpers::{
        common::set_created_by,
        constants::TELEGRAM_STICKER_SET_URL,
        texts::{do_you_want_to_steal, pack_owned_by_user_and_bot, pack_owned_by_user_not_bot},
    },
    presentation::{
        commands::{
            common::{parse_sticker_set_link, yes_no_keyboard_markup},
            states::{add_stickers::AddStickerState, steal_sticker_set::StealStickerSetState},
        },
        telegram_application::{errors::Error as TGError, get_sticker_set_user_id},
    },
};

pub async fn process_sticker_or_link<S: Storage>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
    Extension(client): Extension<Client>,
) -> HandlerResult {
    let set_name: String = if let Some(sticker) = message.sticker() {
        match sticker.set_name() {
            Some(name) => name.to_string(),
            None => return Ok(EventReturn::Skip),
        }
    } else {
        let text = message
            .text()
            .expect("Message is not text. Check the filters");

        if !text.contains(TELEGRAM_STICKER_SET_URL) {
            return Ok(EventReturn::Skip);
        }
        match parse_sticker_set_link(&bot, text).await {
            Ok(name) => name.to_string(),
            Err(_) => return Ok(EventReturn::Skip),
        }
    };

    let bot_username = bot
        .send(GetMe::new())
        .await?
        .username
        .expect("bot without username :/");

    let created_by_bot = set_created_by(&set_name, bot_username.as_ref());

    let user_id = match message.from() {
        Some(u) => u.id,
        None => return Ok(EventReturn::Skip),
    };

    let pack_user_id = match get_sticker_set_user_id(&set_name, &client).await {
        Ok(id) => Some(id),
        Err(TGError::InvocationError(InvocationError::Rpc(err)))
            if err.name == "STICKERSET_INVALID" =>
        {
            None
        }
        Err(err) => {
            error!(
                ?err,
                ?set_name,
                "Error occurred while getting sticker set user id in process_sticker_or_link: "
            );
            return Ok(EventReturn::Skip);
        }
    };
    let owned_by_user = pack_user_id == Some(user_id);

    let chat_id = message.chat().id();
    let message_id = message.message_id();

    if created_by_bot && owned_by_user {
        fsm.set_value("pending_pack_name", &set_name)
            .await
            .map_err(Into::into)?;

        bot.send(
            SendMessage::new(chat_id, pack_owned_by_user_and_bot())
                .reply_markup(yes_no_keyboard_markup())
                .reply_parameters(
                    ReplyParameters::new()
                        .chat_id(chat_id)
                        .message_id(message_id),
                ),
        )
        .await?;

        fsm.set_state(AddStickerState::ConfirmAddStickers)
            .await
            .map_err(Into::into)?;
    } else {
        fsm.set_value("steal_sticker_set_name", &set_name)
            .await
            .map_err(Into::into)?;

        let message = if !created_by_bot && owned_by_user {
            pack_owned_by_user_not_bot()
        } else {
            &do_you_want_to_steal(&set_name)
        };

        bot.send(
            SendMessage::new(chat_id, message)
                .parse_mode(ParseMode::HTML)
                .reply_markup(yes_no_keyboard_markup())
                .reply_parameters(
                    ReplyParameters::new()
                        .chat_id(chat_id)
                        .message_id(message_id),
                ),
        )
        .await?;

        fsm.set_state(StealStickerSetState::DoYouWantToSteal)
            .await
            .map_err(Into::into)?;
    }

    Ok(EventReturn::Finish)
}
