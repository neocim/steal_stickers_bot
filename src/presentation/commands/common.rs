use std::{borrow::Cow, fmt::Display, time::Duration};

use telers::{
    Bot,
    enums::ParseMode,
    errors::{TelegramErrorKind, session::ErrorKind},
    event::{EventReturn, telegram::HandlerResult},
    methods::{AddStickerToSet, SendMessage},
    types::{InputFile, InputSticker, Sticker},
};

use crate::core::helpers::{
    common::sticker_format,
    texts::{default_error_message, detailed_error_message},
};

const SLEEP_TIME: u64 = 1500;

#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum AddStickersError {
    StickerSetInvalid,
    Other(Cow<'static, str>),
}

impl Display for AddStickersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StickerSetInvalid => write!(f, "Sticker set is invalid"),
            Self::Other(err) => write!(f, "Error while adding stickers: {err}"),
        }
    }
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

    for sticker in stickers {
        match add_sticker(bot, user_id, set_name, &sticker).await {
            Ok(_) => {}
            // try to add this sticker again
            Err(AddStickersError::StickerSetInvalid) => {
                for _ in [0..9] {
                    tokio::time::sleep(Duration::from_millis(SLEEP_TIME)).await;
                    if let Ok(_) = add_sticker(bot, user_id, set_name, &sticker).await {
                        break;
                    }
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
