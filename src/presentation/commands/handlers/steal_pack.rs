use telers::{
    Bot, Extension,
    enums::ParseMode,
    errors::{HandlerError, TelegramErrorKind, session::ErrorKind},
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context, Storage},
    methods::{CreateNewStickerSet, DeleteMessage, GetMe, GetStickerSet, SendMessage},
    types::{InputFile, InputSticker, Message, MessageSticker, MessageText},
    utils::text::{html_bold, html_code, html_quote, html_text_link},
};
use tracing::error;

use super::add_stickers::add_stickers;
use crate::{
    application::{
        common::traits::uow::UoWFactory as UoWFactoryTrait, interactors::create_set::create_set,
        set::dto::create::Create as CreateSet,
    },
    core::stickers_helpers::constants::CREATE_SET_IN_ONE_GO_LENGTH_LIMIT,
    presentation::commands::states::steal_sticker_set::StealStickerSetState,
};
use crate::{
    core::stickers_helpers::common::{generate_sticker_set_name_and_link, sticker_format},
    core::texts::sticker_set_message,
};

pub async fn process_non_text(bot: Bot, message: Message) -> HandlerResult {
    bot.send(SendMessage::new(
        message.chat().id(),
        "Please, send me a text message:",
    ))
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn steal_sticker_set_handler<S: Storage>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
) -> HandlerResult {
    fsm.finish().await.map_err(Into::into)?;

    fsm.set_state(StealStickerSetState::StealStickerSetName)
        .await
        .map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        "Send me a sticker and I will steal this sticker pack for you:",
    ))
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn get_sticker_set_name<S: Storage>(
    bot: Bot,
    message: MessageSticker,
    fsm: Context<S>,
) -> HandlerResult {
    let set_name = match message.sticker.set_name {
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

    fsm.set_value("steal_sticker_set_name", set_name.as_ref())
        .await
        .map_err(Into::into)?;

    fsm.set_state(StealStickerSetState::CreateNewStickerSet)
        .await
        .map_err(Into::into)?;

    bot.send(SendMessage::new(
        message.chat.id(),
        "Now enter name for your new sticker pack (1-64 characters):",
    ))
    .await?;

    Ok(EventReturn::Finish)
}

/// ### Panics
/// - Panics if user is unknown (only if message sent in channel)
pub async fn create_new_sticker_set<S, UoWFactory>(
    bot: Bot,
    message: MessageText,
    fsm: Context<S>,
    Extension(uow_factory): Extension<UoWFactory>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    // if user enter wrong sticker set title, process it
    let new_set_title = if message.text.len() > 64 {
        bot.send(SendMessage::new(
            message.chat.id(),
            "Too long name for sticker pack! Please, enter a name up to 64 characters long:",
        ))
        .await?;

        return Ok(EventReturn::Finish);
    } else if message.text.len() < 1 {
        bot.send(SendMessage::new(
            message.chat.id(),
            "Too short name! Please, enter a name between 1 and 64 characters long:",
        ))
        .await?;

        return Ok(EventReturn::Finish);
    } else {
        message.text
    };

    // only panic if i'm forget call fsm.set_value() in function steal_sticker_set_name()
    let steal_sticker_set_name: Box<str> = fsm
        .get_value("steal_sticker_set_name")
        .await
        .map_err(Into::into)?
        .expect("Sticker set name for sticker set user want steal should be set");
    let steal_sticker_set_link = format!("t.me/addstickers/{}", steal_sticker_set_name);

    fsm.finish().await.map_err(Into::into)?;

    let steal_sticker_set = bot
        .send(GetStickerSet::new(steal_sticker_set_name.as_ref()))
        .await?;

    let steal_sticker_set_title = steal_sticker_set.title;

    let steal_stickers_from_sticker_set = steal_sticker_set.stickers;

    // cant panic because bot cant be without username
    let bot_username = bot
        .send(GetMe::new())
        .await?
        .username
        .expect("bot without username :/");

    // only panic if bot using in channels, but i'm using private filter in launch function
    let user_id = message.from.expect("user without id").id;

    // prepare name for new sticker set and link to use it in message later
    let (mut new_set_name, mut new_set_link) =
        generate_sticker_set_name_and_link(11, &bot_username);

    let message_delete = bot.send(SendMessage::new(
        message.chat.id(),
        format!(
            "Stealing sticker pack with name `{title}` for you..\n(stealing sticker packs \
            containing more than {CREATE_SET_IN_ONE_GO_LENGTH_LIMIT} stickers can take up to a several minutes due to some internal limitations)",
            title = html_code(html_quote(&new_set_title))
        ),
    ).parse_mode(ParseMode::HTML))
    .await?;

    let (limit_sticker_set_length, more_than_limit) =
        if steal_stickers_from_sticker_set.len() > CREATE_SET_IN_ONE_GO_LENGTH_LIMIT {
            (CREATE_SET_IN_ONE_GO_LENGTH_LIMIT, true)
        } else {
            (steal_stickers_from_sticker_set.len(), false)
        };

    while let Err(err) = bot
        .send(CreateNewStickerSet::new(
            user_id,
            new_set_name.as_str(),
            new_set_title.as_ref(),
            steal_stickers_from_sticker_set
                .iter()
                .take(limit_sticker_set_length)
                .map(|sticker| {
                    let istick: InputSticker = InputSticker::new(
                        InputFile::id(sticker.file_id.as_ref()),
                        sticker_format(sticker),
                    );
                    istick.emoji_list(sticker.emoji.clone())
                }),
        ))
        .await
    {
        match err {
            ErrorKind::Telegram(err) => {
                if matches!(&err, TelegramErrorKind::BadRequest { message } if message.as_ref()
                    == "Bad Request: SHORTNAME_OCCUPY_FAILED")
                {
                    error!(
                        ?err,
                        "file to create new sticker set; trying to generate sticker set name again:"
                    );
                    error!(new_set_name, "sticker set name:");

                    (new_set_name, new_set_link) =
                        generate_sticker_set_name_and_link(11, &bot_username);
                } else {
                    error!(?err, "error occureded while creating new sticker set:");
                    error!(new_set_name, "sticker set name:");

                    bot.send(SendMessage::new(
                        message.chat.id(),
                        "Error occurded while creating new sticker pack :(",
                    ))
                    .await?;

                    return Ok(EventReturn::Finish);
                }
            }
            err => {
                error!(?err, "error occureded while creating new sticker set:");

                bot.send(SendMessage::new(
                    message.chat.id(),
                    "Error occurded while creating new sticker pack :(",
                ))
                .await?;

                return Ok(EventReturn::Finish);
            }
        }
    }

    let mut uow = uow_factory.create_uow();

    create_set(
        &mut uow,
        CreateSet::new(user_id, new_set_name.as_str(), new_set_title.as_ref()),
    )
    .await
    .map_err(HandlerError::new)?;

    if more_than_limit {
        let all_stickers_was_added = add_stickers(
            &bot,
            user_id,
            new_set_name.as_ref(),
            steal_stickers_from_sticker_set[limit_sticker_set_length..].as_ref(),
        )
        .await
        .expect("empty stickers list");

        if !all_stickers_was_added {
            bot.send(SendMessage::new(
                message.chat.id(),
                format!(
                    "Error occurded while creating new sticker pack {created_pack} (original {original_set}), {but_created}! \n\
                    Due to an error, not all stickers have been stolen :( \
                    (you can delete this sticker pack if you want using the /delpack command in official Telegram bot @Stickers. \
                    Name of this sticker pack: {copy_set_name})",
                    created_pack = html_text_link(html_quote(new_set_title), new_set_link),
                    original_set = html_text_link(html_quote(steal_sticker_set_title), steal_sticker_set_link),
                    but_created = html_bold("but sticker pack was created"),
                    copy_set_name = html_code(new_set_name)
                ),
            ).parse_mode(ParseMode::HTML))
            .await?;

            return Ok(EventReturn::Finish);
        }
    }

    bot.send(
        SendMessage::new(
            message.chat.id(),
            sticker_set_message(
                &new_set_title,
                &new_set_name,
                &new_set_link,
                &steal_sticker_set_title,
                &steal_sticker_set_link,
            ),
        )
        .parse_mode(ParseMode::HTML),
    )
    .await?;

    // delete unnecessary message
    bot.send(DeleteMessage::new(
        message_delete.chat().id(),
        message_delete.id(),
    ))
    .await?;

    Ok(EventReturn::Finish)
}
