use std::time::Duration;

use telers::{
    enums::ParseMode,
    errors::session::ErrorKind,
    event::{telegram::HandlerResult, EventReturn},
    fsm::{Context, Storage},
    methods::{AddStickerToSet, GetStickerSet, SendMessage},
    types::{InputFile, InputSticker, MessageSticker, MessageText, Sticker},
    utils::text::html_text_link,
    Bot,
};

use tracing::{debug, error};

use crate::core::sticker_format;
use crate::AddStickerState;

pub async fn steal_sticker_handler<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        "Send me your sticker pack that stolen by this bot!\n\
        (if you don't have the sticker packs stolen by this bot, first use the command /steal_pack)",
    ))
    .await?;

    fsm.set_state(AddStickerState::GetStolenStickerSet)
        .await
        .map_err(Into::into)?;

    Ok(EventReturn::Finish)
}

pub async fn get_stolen_sticker_set<S: Storage>(
    bot: Bot,
    message: MessageSticker,
    fsm: Context<S>,
) -> HandlerResult {
    let sticker_set_name = match message.sticker.set_name {
        Some(sticker_set_name) => sticker_set_name,
        None => {
            bot.send(SendMessage::new(
                message.chat.id(),
                "This sticker is without the sticker pack! Try send to another sticker pack:",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    let set_length = bot
        .send(GetStickerSet::new(sticker_set_name.as_ref()))
        .await?
        .stickers
        .len();

    if 120 - set_length > 0 {
        bot.send(SendMessage::new(
            message.chat.id(),
            format!("Total length of this sticker pack = {}\nThis means you can add a maximum of {} stickers, \
            otherwise you will get error because the maximum size of a sticker pack in current time = 120 stickers.",
            set_length, 120 - set_length),
        ))
        .await?;
    } else {
        bot.send(SendMessage::new(
                message.chat.id(),
                "Sorry, but this sticker pack contains 120 stickers\nYou cant add more stickers, because the maximum \
                size of a sticker pack in current time = 120 stickers :(\nTry send another pack.",
            ))
            .await?;

        return Ok(EventReturn::Finish);
    }

    fsm.set_value("get_stolen_sticker_set", sticker_set_name)
        .await
        .map_err(Into::into)?;

    fsm.set_state(AddStickerState::GetStickersToAdd)
        .await
        .map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        "Now send me sticker(s) you want to add in stolen sticker pack\n(when you ready, send command /done):",
    ))
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn get_stickers_to_add<S: Storage>(
    bot: Bot,
    message: MessageSticker,
    fsm: Context<S>,
) -> HandlerResult {
    let sticker = message.sticker;

    let sticker_vec: Vec<Sticker> = match fsm
        .get_value::<&str, Vec<Sticker>>("get_stickers_to_add")
        .await
        .map_err(Into::into)?
    {
        Some(mut get_sticker_vec) => {
            let sticker_set_name: Box<str> = fsm
                .get_value("get_stolen_sticker_set")
                .await
                .map_err(Into::into)?
                .expect("Sticker set name for sticker set user want steal should be set");

            let set_length = bot
                .send(GetStickerSet::new(sticker_set_name.as_ref()))
                .await?
                .stickers
                .len();
            let sticker_vec_len = get_sticker_vec.len();

            if set_length + sticker_vec_len >= 120 {
                bot.send(SendMessage::new(
                    message.chat.id(),
                    "Enough stickers! The sum of the current stickers in the sticker pack \
                    and the stickers you want to add to it has reached 120!\nAll next stickers (if you continue sending) \
                    will be ignored!\nPlease, use command /done to add them or /cancel if for some reason you change your mind about adding them.",
                )) 
                .await?;

                return Ok(EventReturn::Finish);
            }

            get_sticker_vec.insert(get_sticker_vec.len(), sticker);

            get_sticker_vec
        }
        None => vec![sticker],
    };

    fsm.set_value("get_stickers_to_add", sticker_vec)
        .await
        .map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        "Sticker processed! Send the next one, or use the /done command if you're ready.",
    ))
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
    // only panic if i'm forget call fsm.set_value() in function get_stolen_sticker_set()
    let sticker_set_name: Box<str> = fsm
        .get_value("get_stolen_sticker_set")
        .await
        .map_err(Into::into)?
        .expect("Sticker set name for sticker set user want steal should be set");

    let stickers_to_add_vec: Vec<Sticker> = match fsm
        .get_value("get_stickers_to_add")
        .await
        .map_err(Into::into)?
    {
        Some(sticker_vec) => sticker_vec,
        None => {
            bot.send(SendMessage::new(
                message.chat.id(),
                "You haven't sent a single sticker! Send the sticker(s), and only then use the /done command:",
            ))
            .await?;

            return Ok(EventReturn::Finish);
        }
    };

    fsm.finish().await.map_err(Into::into)?;

    // only panic if messages uses in channels, but i'm using private filter in main function
    let user_id = message.from.expect("error while parsing user").id;

    bot.send(SendMessage::new(
        message.chat.id(),
        "Done! Trying to add that sticker(s) to your sticker pack..\n\
        (if you have sent a lot of stickers, it may take up to a few minutes to add them)",
    ))
    .await?;

    for sticker_to_add in stickers_to_add_vec {
        if let Err(err) = bot
            .send(AddStickerToSet::new(user_id, sticker_set_name.as_ref(), {
                let sticker_is = InputSticker::new(
                    InputFile::id(sticker_to_add.file_id.as_ref()),
                    sticker_format(&[sticker_to_add.clone()]).expect("stickers not specifed"),
                );

                sticker_is.emoji_list(sticker_to_add.emoji.clone())
            }))
            .await
        {
            match err {
                ErrorKind::Telegram(err) => {
                    if err.to_string() == r#"TelegramBadRequest: "Bad Request: STICKERSET_INVALID""# {
                        error!("file to add sticker: {}", err);
                        debug!("sticker set name: {}", sticker_set_name);
                        debug!("sticker to add: {:?}", sticker_to_add);

                        bot.send(SendMessage::new(
                        message.chat.id(),
                        "Error! I didn't steal this sticker pack!\nTry calling /steal_pack and send me stolen sticker pack.",
                    ))
                    .await?;

                        return Ok(EventReturn::Finish);
                    } else {
                        error!("file to add sticker: {}", err);
                        debug!("sticker set name: {}", sticker_set_name);
                        debug!("sticker to add: {:?}", sticker_to_add);

                        bot.send(SendMessage::new(
                            message.chat.id(),
                            "Error occurded while adding sticker to sticker pack :(\nTry again.",
                        ))
                        .await?;

                        return Ok(EventReturn::Finish);
                    }
                }
                err => {
                    error!(
                        "error occureded while adding sticker sticker set: {}\n",
                        err
                    );
                    debug!("sticker set name: {}", sticker_set_name);

                    bot.send(SendMessage::new(
                        message.chat.id(),
                        "Error occurded while adding sticker to sticker pack :(",
                    ))
                    .await?;

                    return Ok(EventReturn::Finish);
                }
            }
        }

        // sleep because you can’t send telegram api requests more often than per second
        tokio::time::sleep(Duration::from_millis(1010)).await;
    }
    let sticker_set_title = bot
        .send(GetStickerSet::new(sticker_set_name.as_ref()))
        .await?
        .title;

    bot.send(
        SendMessage::new(
            message.chat.id(),
            format!(
                "This sticker was added into {}!",
                html_text_link(
                    sticker_set_title,
                    format!("t.me/addstickers/{}", sticker_set_name)
                )
            ),
        )
        .parse_mode(ParseMode::HTML),
    )
    .await?;

    Ok(EventReturn::Finish)
}