use telers::{
    Bot,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::SendMessage,
    types::Message,
};

use crate::core::helpers::texts::start_message;

pub async fn start_handler<S: Storage>(
    bot: Bot,
    message: Message,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat().id(),
        // only can panic if messages uses in channels, but i'm using private filter in main function
        start_message(&message.from().expect("error while parsing user").first_name),
    ))
    .await?;

    Ok(EventReturn::Finish)
}
