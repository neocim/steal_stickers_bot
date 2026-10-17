use std::{borrow::Cow, fmt::Display, time::Duration};

use telers::{
    Bot,
    enums::ParseMode,
    errors::{HandlerError, TelegramErrorKind, session::ErrorKind},
    event::{EventReturn, telegram::HandlerResult},
    methods::{AddStickerToSet, GetStickerSet, SendMessage},
    types::{InputFile, InputSticker, Message, ReplyParameters, Sticker},
};
use tracing::error;

use crate::core::helpers::{
    common::sticker_format,
    constants::{TELEGRAM_SET_SHORT_NAME_MAX_LENGTH, TELEGRAM_STICKER_SET_URL},
    texts::{default_error_message, detailed_error_message, invalid_set},
};

const SLEEP_TIME: u64 = 1500;

#[derive(Debug, Clone, thiserror::Error)]
pub enum AddStickersError {
    StickerSetInvalid,
    StickerSetDeleted,
    Other(Cow<'static, str>),
}

#[derive(Debug, thiserror::Error)]
#[error("Sticker set link invalid")]
pub struct ParseSetLinkError;

impl Display for AddStickersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StickerSetInvalid => write!(f, "Sticker set is invalid"),
            Self::StickerSetDeleted => write!(f, "It seems the sticker pack has been deleted."),
            Self::Other(err) => write!(f, "Error while adding stickers: {err}"),
        }
    }
}

pub async fn parse_sticker_set_link<'a>(
    bot: &Bot,
    message: &Message,
    text: &str,
) -> Result<String, HandlerError> {
    if let Some(link_idx) = text.find(TELEGRAM_STICKER_SET_URL) {
        let short_name_start = link_idx + TELEGRAM_STICKER_SET_URL.len();

        let set_name: String = text
            .chars()
            .skip(short_name_start)
            .take(TELEGRAM_SET_SHORT_NAME_MAX_LENGTH)
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();

        if set_name.len() == 0 {
            return Err(HandlerError::new(ParseSetLinkError));
        }

        check_set_name(&bot, &message, &set_name).await?;
        return Ok(set_name);
    }
    Err(HandlerError::new(ParseSetLinkError))
}

pub async fn check_set_name(bot: &Bot, message: &Message, set_name: &str) -> HandlerResult {
    if let Err(ref error) = bot.send(GetStickerSet::new(&*set_name)).await {
        if matches!(error, ErrorKind::Telegram(TelegramErrorKind::BadRequest { message }) if **message == *"Bad Request: STICKERSET_INVALID")
        {
            return send_invalid_set_msg(&bot, message.chat().id(), message.message_id()).await;
        }

        error!(
            ?error,
            "Error occurred while getting sticker set name to steal: "
        );

        send_detailed_error_msg(&bot, &error.to_string(), message.chat().id()).await?;

        return Ok(EventReturn::Finish);
    }
    Ok(EventReturn::Finish)
}

pub async fn send_invalid_set_msg(bot: &Bot, chat_id: i64, message_id: i64) -> HandlerResult {
    bot.send(
        SendMessage::new(chat_id, invalid_set()).reply_parameters(
            ReplyParameters::new()
                .chat_id(chat_id)
                .message_id(message_id),
        ),
    )
    .await?;

    return Ok(EventReturn::Finish);
}

pub async fn send_default_error_msg(bot: &Bot, chat_id: i64) -> HandlerResult {
    bot.send(SendMessage::new(chat_id, default_error_message()))
        .await?;

    return Ok(EventReturn::Finish);
}

pub async fn send_detailed_error_msg(bot: &Bot, err: &str, chat_id: i64) -> HandlerResult {
    bot.send(SendMessage::new(chat_id, detailed_error_message(err)).parse_mode(ParseMode::HTML))
        .await?;

    return Ok(EventReturn::Finish);
}

pub async fn add_stickers(
    bot: &Bot,
    user_id: i64,
    set_name: &str,
    stickers: Vec<Sticker>,
) -> Result<(), AddStickersError> {
    if stickers.is_empty() {
        return Err(AddStickersError::Other("List is empty".into()));
    }

    let mut fallback_count = 0;
    for sticker in stickers {
        if fallback_count >= 1 {
            return Err(AddStickersError::StickerSetDeleted);
        }
        match add_sticker(bot, user_id, set_name, &sticker).await {
            Ok(_) => {}
            // try to add this sticker again
            Err(AddStickersError::StickerSetInvalid) => {
                for _ in [0..9] {
                    tokio::time::sleep(Duration::from_millis(SLEEP_TIME)).await;
                    if let Ok(_) = add_sticker(bot, user_id, set_name, &sticker).await {
                        break;
                    }
                    fallback_count += 1;
                }
            }
            Err(err) => return Err(err),
        }
        // sleep because you can’t send telegram api requests more often than per second
        tokio::time::sleep(Duration::from_millis(SLEEP_TIME)).await;
    }

    Ok(())
}

async fn add_sticker<'a>(
    bot: &Bot,
    user_id: i64,
    set_name: &str,
    sticker: &'a Sticker,
) -> Result<(), AddStickersError> {
    match bot
        .send(AddStickerToSet::new(user_id, set_name, {
            let sticker_is = InputSticker::new(
                InputFile::id(sticker.file_id()),
                sticker_format(&sticker),
                sticker.emoji(),
            );

            sticker_is.emoji_list(sticker.emoji().unwrap())
        }))
        .await
    {
        Ok(_) => Ok(()),
        Err(ErrorKind::Telegram(TelegramErrorKind::BadRequest { message }))
            if message.as_ref() == "Bad Request: STICKERSET_INVALID" =>
        {
            Err(AddStickersError::StickerSetInvalid)
        }
        Err(err) => Err(AddStickersError::Other(err.to_string().into())),
    }
}
