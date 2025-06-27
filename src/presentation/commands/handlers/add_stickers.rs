use std::time::Duration;

use grammers_client::Client;
use telers::{
    Bot, Extension,
    enums::ParseMode,
    errors::HandlerError,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::{AddStickerToSet, DeleteMessage, GetMe, GetStickerSet, SendMessage},
    types::{
        InputFile, InputSticker, Message, MessageSticker, MessageText, ReplyParameters, Sticker,
    },
    utils::text::{html_bold, html_quote, html_text_link},
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
        commands::states::add_stickers::AddStickerState,
        telegram_application::get_sticker_set_user_id,
    },
};

pub async fn process_non_sticker_handler(bot: Bot, message: Message) -> HandlerResult {
    bot.send(SendMessage::new(
        message.chat().id(),
        "Please, send me a sticker:",
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
                "Sorry, an erorr occurded. Try send this sticker again :(",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    let sticker_set_title = sticker_set.title;

    let bot_username = bot
        .send(GetMe::new())
        .await?
        .username
        .expect("bot without username :/");

    if !set_created_by(sticker_set_name.as_ref(), bot_username.as_ref()) {
        bot.send(SendMessage::new(
            message.chat.id(),
            "This sticker pack wasnt stolen by me, which means I cant add stickers to it according to Telegram rules. \
            You can see your stolen sticker pack using command /mystickers or steal this sticker pack using command /stealpack.",
        ))
        .await?;

        return Ok(EventReturn::Finish);
    }

    // if function doesnt execute in 3 second, send error message
    let steal_set_user_id = match tokio::time::timeout(Duration::from_secs(10), async {
        let mut error_count: u32 = 0;

        loop {
            match get_sticker_set_user_id(sticker_set_name.as_ref(), &client).await {
                Ok(set_id) => return Ok(set_id),
                Err(err) if error_count >= 5 => return Err(err),
                Err(err) => {
                    error!(
                        ?err,
                        "Error occurded while trying to get sticker set user id:"
                    );

                    error_count += 1;
                }
            }
        }
    })
    .await
    {
        Ok(Ok(set_id)) => set_id,
        Ok(Err(err)) => {
            error!(%err, "Failed to get sticker set user id:");

            bot.send(
                SendMessage::new(
                    message.chat.id(),
                    "Sorry, an error occurded :( Please, try again in few minutes.",
                )
                .reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())),
            )
            .await?;

            return Ok(EventReturn::Finish);
        }
        Err(err) => {
            error!(%err, "Too long time to get sticker set user id:");

            bot.send(
                SendMessage::new(
                    message.chat.id(),
                    "Sorry, an error occurded :( Please, try again in few minutes.",
                )
                .reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())),
            )
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    create_set(
        &mut uow,
        CreateSet::new(
            steal_set_user_id,
            sticker_set_name.as_ref(),
            sticker_set_title.as_ref(),
        ),
    )
    .await
    .map_err(HandlerError::new)?;

    // only panic if messages uses in channels, but i'm using private filter in main function
    let user_id = message.from.expect("user not specified").id;

    if user_id != steal_set_user_id {
        bot.send(
            SendMessage::new(
                message.chat.id(),
                format!(
                    "You are not the owner of this sticker pack! Please, send {your} sticker pack \
            or steal this sticker pack using command /stealpack.",
                    your = html_bold("your stolen")
                ),
            )
            .parse_mode(ParseMode::HTML),
        )
        .await?;

        return Ok(EventReturn::Finish);
    }

    let set_length = bot
        .send(GetStickerSet::new(sticker_set_name.as_ref()))
        .await?
        .stickers
        .len();

    let message_delete = if MAX_STICKER_SET_LENGTH - set_length > 0 {
        bot.send(SendMessage::new(
                message.chat.id(),
                format!("Total length of this sticker pack = {set_length}. This means you can add a maximum of {} stickers, \
                otherwise you will get error because the maximum size of a sticker pack in current time = {MAX_STICKER_SET_LENGTH} stickers.",
                MAX_STICKER_SET_LENGTH - set_length),
            ).reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())))
            .await?
    } else {
        bot.send(SendMessage::new(
                message.chat.id(),
                format!("Sorry, but this sticker pack contains {MAX_STICKER_SET_LENGTH} stickers! :(\n\
                You cant add more stickers, because the maximum size of a sticker pack in current time = {MAX_STICKER_SET_LENGTH} \
                stickers. Try send another pack(or delete some stickers from this sticker pack).")
            ).reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())))
            .await?;

        return Ok(EventReturn::Finish);
    };

    fsm.set_value(
        "get_stolen_sticker_set",
        (sticker_set_name, sticker_set_title, set_length),
    )
    .await
    .map_err(Into::into)?;

    fsm.set_state(AddStickerState::GetStickersToAdd)
        .await
        .map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        "Now send me the sticker(s), you want to add in your sticker pack. \
        When you're ready, use /done command to add all selected stickers into sticker pack.",
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
    Extension(client): Extension<Client>,
    Extension(uow_factory): Extension<UoWFactory>,
    fsm: Context<S>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    let mut uow = uow_factory.create_uow();

    let (_, _, sticker_set_length): (Box<str>, Box<str>, usize) = fsm
        .get_value("get_stolen_sticker_set")
        .await
        .map_err(Into::into)?
        // only panic if i'm forget call fsm.set_value() in function get_stolen_sticker_set()
        .expect("sticker set name and sticker set title for sticker set should be set");

    let sticker_to_add = message.sticker;

    // if sticker belongs to some set, this set can be created in database
    let (sticker_to_add_set_name, set_can_created) = match &sticker_to_add.set_name {
        Some(set_name) => (set_name.as_ref(), true),
        // returning `""`, not `None`, because it will not use if returned value false
        None => ("", false),
    };

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

    if set_can_created {
        // if function doesnt execute in 3 second, send error message
        let sticker_set_owner_id = match tokio::time::timeout(Duration::from_secs(10), async {
            let mut error_count: u32 = 0;

            loop {
                match get_sticker_set_user_id(sticker_to_add_set_name, &client).await {
                    Ok(set_id) => return Ok(set_id),
                    Err(err) if error_count >= 5 => return Err(err),
                    Err(err) => {
                        error!(
                            ?err,
                            "Error number `{error_count}` occurded while trying to get sticker set user id:"
                        );

                        error_count += 1;
                    }
                }
            }
        })
        .await
        {
            Ok(Ok(set_id)) => set_id,
            Ok(Err(err)) => {
                error!(%err, "Failed to get sticker set user id:");

                bot.send(
                    SendMessage::new(
                        message.chat.id(),
                        "Sorry, an error occurded :( Please, try again in few minutes.",
                    )
                    .reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())),
                )
                .await?;

                return Ok(EventReturn::Finish);
            }
            Err(err) => {
                error!(%err, "Too long time to get sticker set user id:");

                bot.send(
                    SendMessage::new(
                        message.chat.id(),
                        "Sorry, an error occurded :( Please, try again in few minutes.",
                    )
                    .reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())),
                )
                .await?;

                return Ok(EventReturn::Finish);
            }
        };
        let sticker_to_add_title = &bot
            .send(GetStickerSet::new(sticker_to_add_set_name))
            .await?
            .title;

        let bot_username = bot
            .send(GetMe::new())
            .await?
            .username
            .expect("bot without username :/");

        if set_created_by(sticker_to_add_set_name, bot_username.as_ref()) {
            create_set(
                &mut uow,
                CreateSet::new(
                    sticker_set_owner_id,
                    sticker_to_add_set_name,
                    sticker_to_add_title,
                ),
            )
            .await
            .map_err(HandlerError::new)?;
        } else {
            // ignore
        }
    }

    let sticker_vec: Vec<Sticker> = match fsm
        .get_value::<&str, Vec<Sticker>>("get_stickers_to_add")
        .await
        .map_err(Into::into)?
    {
        Some(mut sticker_vec) => {
            let sticker_vec_len = sticker_vec.len();

            if sticker_set_length + sticker_vec_len >= MAX_STICKER_SET_LENGTH {
                bot.send(SendMessage::new(
                    message.chat.id(),
                    format!("Please, use /done to add stickers, because the amount of stickers has reached \
                    {MAX_STICKER_SET_LENGTH}. All next stickers (if you'll continue sending) will be ignored."),
                ))
                .await?;

                return Ok(EventReturn::Finish);
            }

            sticker_vec.push(sticker_to_add);

            sticker_vec
        }
        None => vec![sticker_to_add],
    };

    fsm.set_value("get_stickers_to_add", sticker_vec)
        .await
        .map_err(Into::into)?;

    bot.send(
        SendMessage::new(
            message.chat.id(),
            "Sticker processed! Send the next one, or use the /done command if you're ready.",
        )
        .reply_parameters(ReplyParameters::new(message.id).chat_id(message.chat.id())),
    )
    .await?;

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

    let stickers: Vec<Sticker> = match fsm
        .get_value("get_stickers_to_add")
        .await
        .map_err(Into::into)?
    {
        Some(sticker_vec) => sticker_vec,
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
            "Error occurded while adding stickers. Due to an error, not all specified stickers have been added into {set} :(",
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
