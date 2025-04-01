use std::{borrow::Cow, time::Duration};

use telers::{
    event::{telegram::HandlerResult, EventReturn},
    methods::{AddStickerToSet, SendMessage},
    types::{InputFile, InputSticker, Message, Sticker},
    Bot,
};
use tracing::error;

use crate::core::stickers_helpers::common::sticker_format;

#[derive(Debug, Clone, thiserror::Error)]
#[error("Error occurded while adding stickers: {message}")]
pub struct AddStickersError {
    message: Cow<'static, str>,
}

impl AddStickersError {
    fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub async fn process_non_sticker(bot: Bot, message: Message) -> HandlerResult {
    bot.send(SendMessage::new(
        message.chat().id(),
        "Please, send me a sticker.",
    ))
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn add_stickers(
    bot: &Bot,
    user_id: i64,
    set_name: &str,
    sticker_list: &[Sticker],
) -> Result<bool, AddStickersError> {
    if sticker_list.is_empty() {
        return Err(AddStickersError::new("list is empty"));
    }

    let mut all_stickers_was_stolen = true;

    let mut sticker_formats = sticker_list.iter().map(|sticker| sticker_format(sticker));

    for sticker in sticker_list {
        if let Err(err) = bot
            .send(AddStickerToSet::new(user_id, set_name, {
                let sticker_is = InputSticker::new(
                    InputFile::id(sticker.file_id.as_ref()),
                    sticker_formats.next().unwrap(),
                );

                sticker_is.emoji_list(sticker.clone().emoji)
            }))
            .await
        {
            error!(?err, "error occureded while adding sticker to sticker set:");
            error!(set_name, "sticker set name:");

            all_stickers_was_stolen = false;
        }

        // sleep because you can’t send telegram api requests more often than per second
        tokio::time::sleep(Duration::from_millis(1001)).await;
    }

    Ok(all_stickers_was_stolen)
}
