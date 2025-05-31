use std::borrow::Cow;

use telers::{
    Bot, Extension,
    enums::ParseMode,
    errors::HandlerError,
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context as FSMContext, Storage},
    methods::{AnswerCallbackQuery, EditMessageText, SendMessage},
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, MessageText, ReplyMarkup},
};
use tracing::{error, warn};

use crate::{
    application::{
        common::{
            exceptions::BeginError,
            traits::uow::{UoW as _, UoWFactory as UoWFactoryTrait},
        },
        set::{dto::get_by_tg_id::GetByTgID as GetSetByTgID, repository::SetRepo as _},
    },
    core::{
        stickers_helpers::constants::STICKER_SETS_NUMBER_PER_PAGE, texts::current_page_message,
    },
    domain::entities::set::Set,
    presentation::commands::states::callback_data::CallbackData,
};

impl From<BeginError> for HandlerError {
    fn from(value: BeginError) -> Self {
        HandlerError::new(value)
    }
}

#[derive(Debug, Clone)]
struct GetButtonsError {
    message: Cow<'static, str>,
}

impl GetButtonsError {
    fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub async fn my_stickers_handler<S, UoWFactory>(
    bot: Bot,
    message: MessageText,
    fsm: FSMContext<S>,
    Extension(uow_factory): Extension<UoWFactory>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    fsm.finish().await.map_err(Into::into)?;
    let mut uow = uow_factory.create_uow();
    let chat_id = message.chat.id();

    let sticker_sets = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .get_by_tg_id(GetSetByTgID::new(
            // panics if using not in private chats, but i use filter
            message.from.expect("Failed to get user id").id,
            Some(false),
        ))
        .await
        .map_err(HandlerError::new)?;

    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    let number_of_pages =
        match get_buttons(&sticker_sets, STICKER_SETS_NUMBER_PER_PAGE, &mut buttons) {
            Ok(pages) => pages,
            Err(err) => {
                bot.send(SendMessage::new(chat_id, err.message.to_string()))
                    .await?;

                return Ok(EventReturn::Finish);
            }
        };

    let reply_markup = ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(buttons));
    bot.send(
        SendMessage::new(
            chat_id,
            current_page_message(
                1,
                number_of_pages,
                STICKER_SETS_NUMBER_PER_PAGE,
                sticker_sets.as_ref(),
            ),
        )
        .parse_mode(ParseMode::HTML)
        .reply_markup(reply_markup.clone()),
    )
    .await?;

    Ok(EventReturn::Finish)
}

pub async fn process_button<S, UoWFactory>(
    bot: Bot,
    callback_query: CallbackQuery,
    fsm: FSMContext<S>,
    Extension(uow_factory): Extension<UoWFactory>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
    S: Storage,
{
    let mut uow = uow_factory.create_uow();

    let (chat_id, message_id) = match (callback_query.chat_id(), callback_query.message_id()) {
        (Some(chat_id), Some(message_id)) => (chat_id, message_id),
        _ => return Ok(EventReturn::Finish),
    };

    bot.send(AnswerCallbackQuery::new(callback_query.id))
        .await?;

    let mut message_data = match &callback_query.data {
        Some(message_data) => message_data.chars(),
        None => {
            error!(
                "None value occurded while processed callback query from inline keyboard button!"
            );
            bot.send(SendMessage::new(chat_id, "Internal error"))
                .await?;
            return Ok(EventReturn::Finish);
        }
    };

    message_data
        .nth(CallbackData::MyStickers.as_str().len() - 1)
        .expect("Failed to eat callback data prefix");

    let current_page_number = match message_data.as_str().parse::<usize>() {
        Ok(page_number) => page_number,
        Err(_) => return Ok(EventReturn::Finish),
    };

    // process if user click to one button several times in a row
    match fsm
        .get_value::<_, Box<str>>("previous_callback_query")
        .await
        .map_err(Into::into)?
    {
        Some(msg_data) if &*msg_data == message_data.as_str() => return Ok(EventReturn::Finish),
        _ => {
            fsm.set_value("previous_callback_query", Some(message_data.as_str()))
                .await
                .map_err(Into::into)?;
        }
    };

    let sticker_sets = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .get_by_tg_id(GetSetByTgID::new(callback_query.from.id, Some(false)))
        .await
        .map_err(HandlerError::new)?;

    let mut buttons = Vec::new();
    let number_of_pages =
        match get_buttons(&sticker_sets, STICKER_SETS_NUMBER_PER_PAGE, &mut buttons) {
            Ok(pages) => pages,
            Err(err) => {
                bot.send(SendMessage::new(chat_id, err.message.to_string()))
                    .await?;

                return Ok(EventReturn::Finish);
            }
        };
    if number_of_pages == 1 {
        return Ok(EventReturn::Finish);
    }

    let inline_keyboard_markup = InlineKeyboardMarkup::new(buttons);
    let edit_message = EditMessageText::new(current_page_message(
        current_page_number,
        number_of_pages,
        STICKER_SETS_NUMBER_PER_PAGE,
        &sticker_sets,
    ))
    .chat_id(chat_id)
    .message_id(message_id)
    .reply_markup(inline_keyboard_markup);

    bot.send(edit_message.parse_mode(ParseMode::HTML)).await?;

    Ok(EventReturn::Finish)
}

fn get_buttons(
    list: &[Set],
    sticker_sets_number_per_page: usize,
    buttons: &mut Vec<Vec<InlineKeyboardButton>>,
) -> Result<u32, GetButtonsError> {
    let mut page_count: u32 = 0;
    let mut current_row_index = 0;

    if list.len() > sticker_sets_number_per_page || !list.is_empty() {
        list.iter()
            .enumerate()
            .filter(|(index, _)| index % sticker_sets_number_per_page == 0)
            .for_each(|_| {
                // create a new row every 5 buttons
                if page_count % 5 == 0 {
                    page_count += 1;
                    current_row_index += 1;

                    buttons.push(vec![
                        InlineKeyboardButton::new(format!("Page {page_count}",)).callback_data(
                            format!(
                                "{prefix}{page_count}",
                                prefix = CallbackData::MyStickers.as_str()
                            ),
                        ),
                    ])
                // else push button into current row
                } else {
                    page_count += 1;

                    buttons[current_row_index - 1].push(
                        InlineKeyboardButton::new(format!("Page {page_count}",)).callback_data(
                            format!(
                                "{prefix}{page_count}",
                                prefix = CallbackData::MyStickers.as_str()
                            ),
                        ),
                    );
                }
            })
    // otherwise user does not have sticker sets stolen by this bot
    } else {
        return Err(GetButtonsError::new(
            "You don't have a single stolen sticker pack. \
            Steal any sticker pack using the /stealpack command and it will appear in this list.",
        ));
    };

    Ok(page_count)
}
