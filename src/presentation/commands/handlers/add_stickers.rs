use std::time::Duration;

use grammers_client::Client;
use telers::{
    enums::ParseMode, 
    errors::{session::ErrorKind, HandlerError, TelegramErrorKind}, 
    event::{telegram::HandlerResult, EventReturn}, fsm::{Context, Storage}, 
    methods::{AddStickerToSet, DeleteMessage, GetMe, GetStickerSet, SendMessage, SendSticker}, 
    types::{InputFile, InputFileId, InputSticker, Message, MessageSticker, MessageText, ReplyParameters, Sticker}, 
    utils::text::{html_code, html_quote, html_text_link}, Bot, Extension
};
use tracing::error;

use super::AddStickersError;
use crate::{
    application::{
        common::traits::uow::UoWFactory as UoWFactoryTrait, interactors::create_set::create_set,
        set::dto::create::Create as CreateSet,
    },
    core::helpers::{
        common::{set_created_by, sticker_format},
        constants::{MAX_STICKER_SET_LENGTH, TELEGRAM_STICKER_SET_URL},
    },
    presentation::{
        commands::{states::add_stickers::AddStickerState},
        telegram_application::get_sticker_set_user_id,
    },
};

pub async fn process_non_sticker_handler(bot: Bot, message: Message) -> HandlerResult {
    bot.send(SendMessage::new(
        message.chat().id(),
        "Please, send me a sticker.",
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
            "Send me your stolen sticker pack, in which you want to add sticker(s). \
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
    Extension(uow_factory): Extension<UoWFactory>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    let mut uow = uow_factory.create_uow();

    let sticker_set_name = match message.sticker.set_name {
        Some(sticker_set_name) => sticker_set_name,
        None => {
            bot.send(SendMessage::new(
                message.chat.id(),
                "This sticker is without sticker pack! Try to send another sticker pack.",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    if let Err(ref error) = bot.send(GetStickerSet::new(&*sticker_set_name)).await {
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
            "Error occurded while getting sticker set name to steal: "
        );

        bot.send(SendMessage::new(
            message.chat.id(),
            "Sorry, an error occurded.",
        ))
        .await?;

        return Ok(EventReturn::Finish);
    }

    let sticker_set = match bot
        .send(GetStickerSet::new(sticker_set_name.as_ref()))
        .await
    {
        Ok(set) => set,
        Err(err) => {
            error!(
                ?err,
                "Error occurded while getting sticker set to add stickers into it:"
            );

            bot.send(SendMessage::new(
                message.chat.id(),
                "Sorry, an erorr occurded.",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    let bot_username = bot
        .send(GetMe::new())
        .await?
        .username
        .expect("bot without username :/");

    if !set_created_by(sticker_set_name.as_ref(), bot_username.as_ref()) {
        bot.send(SendMessage::new(
            message.chat.id(),
            "This sticker pack wasnt stolen by me, which means I cant add stickers to it according to Telegram rules. \
            You can see all your stolen sticker pack using command /mystickers or steal this sticker pack using command /stealpack.",
        ))
        .await?;

        return Ok(EventReturn::Finish);
    }

    let sticker_set_user_id = match get_sticker_set_user_id(&sticker_set_name, &client).await {
        Ok(id) => id,
        Err(error) => {
            error!(?error, "Error occurded while getting sticker set user id: ");

            bot.send(SendMessage::new(message.chat.id(), "Sorry, an error occurded")
                .reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())),).await?;

            return Ok(EventReturn::Finish);
        }
    };

    create_set(
        &mut uow,
        CreateSet::new(
            sticker_set_user_id,
            sticker_set_name.as_ref(),
            sticker_set.title.as_ref(),
        ),
    )
    .await
    .map_err(HandlerError::new)?;

    // only panic if messages uses in channels, but i'm using private filter in main function
    let user_id = message.from.expect("user not specified").id;

    if user_id != sticker_set_user_id {
        bot.send(
            SendMessage::new(
                message.chat.id(),
                "You are not the owner of this sticker pack! Please, send your sticker pack, stolen by me \
                or steal this sticker pack using command /stealpack.",
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
                format!("This sticker pack is completely filled. If you need to, remove a few stickers from it and only then \
                use this command again.")
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
        "Now send me the sticker(s), you want to add in your sticker pack. \
        When you're ready, use /done command to add all selected stickers into sticker pack. \
        Also you can remove last sent sticker from the add list, using /undo command.",
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
                    format!("Please, use /done to add stickers, because the amount of stickers has reached \
                    {max_len}. All next stickers (if you'll continue sending) will be ignored.", 
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

    bot.send(SendMessage::new(message.chat.id(), "This sticker has been removed.")
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
                "You've removed all the stickers. Send the sticker(s), and only then use /done command.",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
        Some(stickers_vec) => stickers_vec,
        None => {
            bot.send(SendMessage::new(
                message.chat.id(),
                "You haven't sent a single sticker! Send the sticker(s), and only then use the /done command.",
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
            "Error occurded while adding stickers into {set}. Due to an error, not all specified stickers have been added.",
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
            error!(?err, "Error occureded while adding sticker to sticker set:");
            error!(set_name, "Sticker set name:");

            all_stickers_was_stolen = false;
        }

        // sleep because you can’t send telegram api requests more often than per second
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }

    Ok(all_stickers_was_stolen)
}
