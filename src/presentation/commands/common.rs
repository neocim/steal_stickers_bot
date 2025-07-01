use std::{borrow::Cow, time::Duration};

use telers::{
    Bot,
    event::{EventReturn, telegram::HandlerResult},
    methods::{AddStickerToSet, SendMessage},
    types::{InputFile, InputSticker, Sticker},
};
use tracing::error;

use crate::core::helpers::{common::sticker_format, texts::default_error_message};

#[derive(Debug, Clone, thiserror::Error)]
#[error("Error occurred while adding stickers: {message}")]
pub(crate) struct AddStickersError {
    message: Cow<'static, str>,
}

impl AddStickersError {
    fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub async fn send_default_error_message(bot: &Bot, chat_id: i64) -> HandlerResult {
    bot.send(SendMessage::new(chat_id, default_error_message()))
        .await?;

    return Ok(EventReturn::Finish);
}

/// Returns `true` if all stickers was stolen, `false` otherwise
pub async fn add_stickers(
    bot: &Bot,
    user_id: i64,
    set_name: &str,
    stickers: Vec<Sticker>,
) -> Result<bool, AddStickersError> {
    if stickers.is_empty() {
        return Err(AddStickersError::new("list is empty"));
    }

    let mut all_stickers_was_stolen = true;
    for sticker in stickers {
        if let Err(err) = bot
            .send(AddStickerToSet::new(user_id, set_name, {
                let sticker_is = InputSticker::new(
                    InputFile::id(sticker.file_id.as_ref()),
                    sticker_format(&sticker),
                );

                sticker_is.emoji_list(sticker.emoji)
            }))
            .await
        {
            error!(
                ?err,
                ?set_name,
                "Error occurred while adding stickers to sticker set: "
            );
            all_stickers_was_stolen = false;
        }

        // sleep because you can’t send telegram api requests more often than per second
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }

    Ok(all_stickers_was_stolen)
}
