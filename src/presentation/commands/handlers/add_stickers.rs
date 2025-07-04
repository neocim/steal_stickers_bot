use std::time::Duration;

use grammers_client::Client;
use telers::{
    enums::ParseMode, 
    errors::{session::ErrorKind, TelegramErrorKind}, 
    event::{telegram::HandlerResult, EventReturn}, fsm::{Context, Storage}, 
    methods::{DeleteMessage, GetMe, GetStickerSet, SendMessage, SendSticker}, 
    types::{InputFile, InputFileId, Message, MessageSticker, MessageText, ReplyParameters, Sticker}, 
    utils::text::{html_code, html_quote, html_text_link}, Bot, Extension
};
use tracing::error;

use crate::{
    application::{
        common::traits::uow::UoWFactory as UoWFactoryTrait,
    },
    core::helpers::{
        common::set_created_by,
        constants::{MAX_STICKER_SET_LENGTH, TELEGRAM_STICKER_SET_URL},
    },
    presentation::{
        commands::{common::{add_stickers, send_default_error_message}, states::add_stickers::AddStickerState},
        telegram_application::get_sticker_set_user_id,
    },
};

pub async fn process_non_sticker_handler(bot: Bot, message: Message) -> HandlerResult {
    bot.send(SendMessage::new(
        message.chat().id(),
        "Please send me a sticker.",
    ))
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn add_stickers_handler<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    fsm.set_state(AddStickerState::GetStolenStickerSet)
        .await
        .map_err(Into::into)?;

    bot.send(
        SendMessage::new(
            message.chat.id(),
            "Send me your stolen sticker pack, in which you want to add stickers. \
                You can see all your stolen sticker packs, using command /mystickers.",
        )
        .parse_mode(ParseMode::HTML),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn get_stolen_sticker_set<S, UoWFactory>(
    bot: Bot,
    message: MessageSticker,
    fsm: Context<S>,
    Extension(client): Extension<Client>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    let sticker_set_name = match message.sticker.set_name {
        Some(sticker_set_name) => sticker_set_name,
        None => {
            bot.send(SendMessage::new(
                message.chat.id(),
                "This sticker is without sticker pack. Try to send another sticker pack.",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    let result = bot.send(GetStickerSet::new(&*sticker_set_name)).await;

    if let Err(ref error) = result {
        if matches!(error, ErrorKind::Telegram(TelegramErrorKind::BadRequest { message }) if **message == *"Bad Request: STICKERSET_INVALID")
        {
            bot.send(SendMessage::new(
                message.chat.id(),
                "This sticker is without sticker pack. Try to send another sticker pack.",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }

        error!(
            ?error,
            "Error occurred while getting sticker set name to steal: "
        );

        send_default_error_message(&bot, message.chat.id()).await?;

        return Ok(EventReturn::Finish);
    }

    // we checked above for errors
    let sticker_set = result.unwrap();

    let bot_username = bot
        .send(GetMe::new())
        .await?
        .username
        .expect("bot without username :/");

    if !set_created_by(sticker_set_name.as_ref(), bot_username.as_ref()) {
        bot.send(SendMessage::new(
            message.chat.id(),
            "This sticker pack wasn't stolen by me, which means that I cannot add stickers to it according to Telegram rules. \
            You can view all your stolen stickers using /mystickers or steal this pack using /stealpack.",
        ))
        .await?;

        return Ok(EventReturn::Finish);
    }

    let sticker_set_user_id = match get_sticker_set_user_id(&sticker_set_name, &client).await {
        Ok(id) => id,
        Err(error) => {
            error!(?error, ?sticker_set_name, "Error occurred while getting sticker set user id: ");

            send_default_error_message(&bot, message.chat.id()).await?;

            return Ok(EventReturn::Finish);
        }
    };

    // only panic if messages uses in channels, but i'm using private filter
    let user_id = message.from.expect("user not specified").id;

    if user_id != sticker_set_user_id {
        bot.send(
            SendMessage::new(
                message.chat.id(),
                "You are not the owner of this sticker pack. \
                Please send me your sticker pack that stolen by me, or steal this pack using /stealpack.",
            )
            .parse_mode(ParseMode::HTML),
        )
        .await?;

        return Ok(EventReturn::Finish);
    }

    let set_length = sticker_set.stickers.len();

    let message_delete = if MAX_STICKER_SET_LENGTH - set_length > 0 {
        bot.send(
            SendMessage::new(
                message.chat.id(),
                format!("Current length of this sticker pack is {set_length_code}. You can add {remaining} more stickers.",
                set_length_code = html_code(set_length.to_string()), 
                remaining = html_code((MAX_STICKER_SET_LENGTH - set_length).to_string())
            )).parse_mode(ParseMode::HTML)
            .reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())),
    )
        .await?
    } else {
        bot.send(SendMessage::new(
                message.chat.id(),
                format!("This sticker pack is completely filled. \
                Remove a few stickers from it and only then use /addstickers again.")
            ).reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())))
            .await?;

        return Ok(EventReturn::Finish);
    };

    fsm.set_value(
        "get_stolen_sticker_set",
        (sticker_set_name, sticker_set.title, set_length),
    )
    .await
    .map_err(Into::into)?;

    fsm.set_state(AddStickerState::GetStickersToAdd)
        .await
        .map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        "Now send me the stickers that you want to add to your sticker pack. \
        When you're ready, use /done to add all selected stickers to the sticker pack. \
        You can also remove last sent sticker from the add list using /undo.",
    ))
    .await?;

    // delete unnecessary message after 15 sec
    tokio::time::sleep(Duration::from_secs(15)).await;
    bot.send(DeleteMessage::new(
        message_delete.chat().id(),
        message_delete.id(),
    ))
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn get_stickers_to_add<S, UoWFactory>(
    bot: Bot,
    message: MessageSticker,
    fsm: Context<S>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    let (_, _, sticker_set_length): (Box<str>, Box<str>, usize) = fsm
        .get_value("get_stolen_sticker_set")
        .await
        .map_err(Into::into)?
        // only panic if i'm forget call fsm.set_value() in function get_stolen_sticker_set()
        .expect("sticker set name and sticker set title for sticker set should be set");

    let sticker_to_add = message.sticker;

    if sticker_to_add.emoji.is_none() {
        bot.send(
            SendMessage::new(
                message.chat.id(),
                "Sorry, but this sticker is without emoji. Try send another sticker.",
            )
            .reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())),
        )
        .await?;

        return Ok(EventReturn::Finish);
    }

    let stickers_vec: Vec<Sticker> = match fsm
        .get_value::<&str, Vec<Sticker>>("get_stickers_to_add")
        .await
        .map_err(Into::into)?
    {
        Some(mut stickers_vec) => {
            let stickers_vec_len = stickers_vec.len();

            if sticker_set_length + stickers_vec_len >= MAX_STICKER_SET_LENGTH {
                bot.send(SendMessage::new(
                    message.chat.id(),
                    format!("The amount of stickers has reached {max_len}. Use /done to add all the selected stickers, or 
                    /undo if you want to remove the latest stickers from the add list. All the following sent stickers will be ignored.", 
                    max_len = html_code(MAX_STICKER_SET_LENGTH.to_string())),
                ))
                .await?;

                return Ok(EventReturn::Finish);
            }

            stickers_vec.push(sticker_to_add);
            stickers_vec
        }
        None => vec![sticker_to_add],
    };

    fsm.set_value("get_stickers_to_add", stickers_vec)
        .await
        .map_err(Into::into)?;

    bot.send(
        SendMessage::new(
            message.chat.id(),
            "Sticker processed! Send the next one or use the /done or /undo commands.",
        )
        .reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn undo_last_sticker<S: Storage>(bot: Bot, message: MessageText, fsm: Context<S>) -> HandlerResult {
    let mut stickers_vec: Vec<Sticker> = match fsm
        .get_value("get_stickers_to_add")
        .await
        .map_err(Into::into)?
    {
        Some(stickers_vec) => stickers_vec,
        None => {
            bot.send(SendMessage::new(
                message.chat.id(),
                "There is nothing to remove.",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    let sticker = match stickers_vec.pop() {
        Some(sticker) => sticker,
        None => {
            bot.send(SendMessage::new(message.chat.id(), "There is nothing to remove.")).await?;

            return Ok(EventReturn::Finish);
        }
    };
    
    fsm.set_value("get_stickers_to_add", stickers_vec).await.map_err(Into::into)?;

    let sticker_message = bot.send(SendSticker::new(message.chat.id(), 
    InputFile::Id(InputFileId::new(&*sticker.file_id)))).await?;

    bot.send(SendMessage::new(message.chat.id(), 
    "This sticker has been removed. \
        You can try using /done or /undo again.")
        .reply_parameters(
            ReplyParameters::new(
                sticker_message.id())
            ).chat_id(sticker_message.chat().id())
        ).await?;

    Ok(EventReturn::Finish)
}

/// ### Panics
/// - Panics if user is unknown (only if message sent in channel)
pub async fn add_stickers_to_user_owned_sticker_set<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    let (sticker_set_name, sticker_set_title, _): (Box<str>, Box<str>, usize) = fsm
        .get_value("get_stolen_sticker_set")
        .await
        .map_err(Into::into)?
        // only panic if i'm forget call fsm.set_value() in function get_stolen_sticker_set()
        .expect("Sticker set name for sticker set should be set");

    let stickers = match fsm
        .get_value::<_, Vec<Sticker>>("get_stickers_to_add")
        .await
        .map_err(Into::into)?
    {
        Some(stickers_vec) if stickers_vec.len() == 0 => { bot.send(SendMessage::new(
                message.chat.id(),
                "You've removed all the stickers. Send the stickers, and only then use /done command.",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
        Some(stickers_vec) => stickers_vec,
        None => {
            bot.send(SendMessage::new(
                message.chat.id(),
                "You haven't sent a single sticker! Send the stickers, and only then use the /done command.",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    fsm.finish().await.map_err(Into::into)?;

    // only panic if messages uses in channels, but i'm using private filter in main function
    let user_id = message.from.expect("Error while parsing user").id;

    let message_delete = bot
        .send(
            SendMessage::new(
                message.chat.id(),
                format!(
                "Done! Trying to add that sticker(s) into {your} sticker pack.. \
                It may take up to a several minutes, if you have selected a lot of stickers to add.",
                your = html_text_link("your", &sticker_set_name)),
            )
            .parse_mode(ParseMode::HTML),
        )
        .await?;

    let all_stickers_was_added = add_stickers(&bot, user_id, sticker_set_name.as_ref(), stickers)
        .await
        // cant panic because we checked above that we're have at least 1 sticker in this list
        .expect("empty stickers list");

    // delete unnecessary message
    bot.send(DeleteMessage::new(
        message_delete.chat().id(),
        message_delete.id(),
    ))
    .await?;

    let stickers_was_added_msg = if all_stickers_was_added {
        format!(
            "Sticker(s) have been added into {set}!",
            set = html_text_link(
                html_quote(sticker_set_title),
                format!("{TELEGRAM_STICKER_SET_URL}{}", sticker_set_name)
            )
        )
    } else {
        format!(
            "Error occurred while adding stickers into {set}. Due to an error, not all specified stickers have been added.",
            set = html_text_link(
                html_quote(sticker_set_title),
                format!("{TELEGRAM_STICKER_SET_URL}{}", sticker_set_name)
            )
        )
    };

    bot.send(
        SendMessage::new(message.chat.id(), stickers_was_added_msg).parse_mode(ParseMode::HTML),
    )
    .await?;

    Ok(EventReturn::Finish)
}
