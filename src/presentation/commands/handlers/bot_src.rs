use telers::{
    Bot,
    enums::ParseMode,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::SendMessage,
    types::MessageText,
    utils::text::html_text_link,
};

pub async fn source_handler<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    bot.send(
        SendMessage::new(
            message.chat.id(),
            format!(
                "Source code of the bot {here}! You can also star it :)",
                here = html_text_link("here", "github.com/neocim/steal_stickers_bot")
            ),
        )
        .parse_mode(ParseMode::HTML),
    )
    .await?;

    Ok(EventReturn::Finish)
}
