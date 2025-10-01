use telers::{
    Bot,
    event::{EventReturn, telegram::HandlerResult},
    methods::SendMessage,
    types::Message,
};

pub async fn process_non_sticker_handler(bot: Bot, message: Message) -> HandlerResult {
    bot.send(SendMessage::new(
        message.chat().id(),
        "Please send me a sticker.",
    ))
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn process_non_text_handler(bot: Bot, message: Message) -> HandlerResult {
    bot.send(SendMessage::new(
        message.chat().id(),
        "Please send me a text message.",
    ))
    .await?;

    Ok(EventReturn::Finish)
}
