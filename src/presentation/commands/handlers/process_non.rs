use telers::{
    Bot,
    event::{EventReturn, telegram::HandlerResult},
    methods::SendMessage,
    types::{Message, ReplyParameters},
};

use crate::core::helpers::texts::link_or_sticker;

pub async fn process_non_sticker_handler(bot: Bot, message: Message) -> HandlerResult {
    bot.send(
        SendMessage::new(message.chat().id(), "Please send a sticker.").reply_parameters(
            ReplyParameters::new()
                .chat_id(message.chat().id())
                .message_id(message.message_id()),
        ),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn process_non_text_handler(bot: Bot, message: Message) -> HandlerResult {
    bot.send(
        SendMessage::new(message.chat().id(), "Please send a text message.").reply_parameters(
            ReplyParameters::new()
                .chat_id(message.chat().id())
                .message_id(message.message_id()),
        ),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn process_non_text_or_stickers_handler(bot: Bot, message: Message) -> HandlerResult {
    bot.send(
        SendMessage::new(message.chat().id(), link_or_sticker()).reply_parameters(
            ReplyParameters::new()
                .chat_id(message.chat().id())
                .message_id(message.message_id()),
        ),
    )
    .await?;

    Ok(EventReturn::Finish)
}
