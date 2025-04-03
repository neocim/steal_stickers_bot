use telers::{
    Bot,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::SendMessage,
    types::MessageText,
};

pub async fn cancel_handler<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        "Last command was canceled.",
    ))
    .await?;

    Ok(EventReturn::Finish)
}
