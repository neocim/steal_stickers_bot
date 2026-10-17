use std::borrow::Cow;

use telers::{
    Bot, Extension,
    callback_data::CallbackData as _,
    enums::ParseMode,
    errors::{HandlerError, TelegramErrorKind, session::ErrorKind},
    event::{EventReturn, telegram::HandlerResult},
    fsm::{Context as FSMContext, Storage},
    methods::{AnswerCallbackQuery, EditMessageText, SendMessage},
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, MessageText, ReplyMarkup},
};

use crate::{
    application::{
        common::{
            exceptions::BeginError,
            traits::uow::{UoW as _, UoWFactory as UoWFactoryTrait},
        },
        set::{dto::get_by_tg_id::GetByTgID as GetSetByTgID, repository::SetRepo as _},
    },
    core::helpers::{
        constants::STICKER_SETS_NUMBER_PER_PAGE,
        texts::{current_page_message, user_doesnt_has_stickers_yet},
    },
    domain::entities::set::Set,
    presentation::commands::{
        common::send_detailed_error_msg, states::callback_data::MyStickersPage,
    },
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

    let reply_markup = ReplyMarkup::InlineKeyboardMarkup(InlineKeyboardMarkup::new(buttons));
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
pub async fn process_buttons<UoWFactory>(
    bot: Bot,
    callback_query: CallbackQuery,
    page: MyStickersPage, // <- извлекается фильтром CallbackData
    Extension(uow_factory): Extension<UoWFactory>,
) -> HandlerResult
where
    UoWFactory: UoWFactoryTrait,
{
    let mut uow = uow_factory.create_uow();

    let (chat_id, message_id) = if let Some(message) = callback_query.message {
        (message.chat().id(), message.message_id())
    } else {
        return Ok(EventReturn::Finish);
    };

    let current_page_number = page.page;

    let sticker_sets = uow
        .set_repo()
        .await
        .map_err(HandlerError::new)?
        .get_by_tg_id(GetSetByTgID::new(callback_query.from.id, Some(false)))
        .await
        .map_err(HandlerError::new)?;

    bot.send(AnswerCallbackQuery::new(callback_query.id))
        .await?;

    let mut buttons = Vec::new();
    let number_of_pages =
        match get_buttons(&sticker_sets, STICKER_SETS_NUMBER_PER_PAGE, &mut buttons) {
            Ok(pages) => pages,
            Err(err) => {
                send_detailed_error_msg(&bot, &err.message.to_string(), chat_id, message_id)
                    .await?;
                return Ok(EventReturn::Finish);
            }
        };

    if number_of_pages == 1 {
        return Ok(EventReturn::Finish);
    }

    let inline_keyboard_markup = InlineKeyboardMarkup::new(buttons);
    let edit_message = EditMessageText::new()
        .text(current_page_message(
            current_page_number,
            number_of_pages,
            STICKER_SETS_NUMBER_PER_PAGE,
            &sticker_sets,
        ))
        .chat_id(chat_id)
        .message_id(message_id)
        .reply_markup(inline_keyboard_markup);

    if let Err(error) = bot.send(edit_message.parse_mode(ParseMode::HTML)).await {
        match &error {
            ErrorKind::Telegram(TelegramErrorKind::BadRequest { message }) => {
                // we need to ignore this bad request error
                if !message.contains("message is not modified") {
                    return Err(error.into());
                }
            }
            _ => return Err(error.into()),
        }
    }

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
                let page = page_count + 1;
                let button = InlineKeyboardButton::new(format!("Page {page}")).callback_data(
                    MyStickersPage {
                        page: page as usize,
                    }
                    .pack()
                    .expect("MyStickersPage is always packable"),
                );

                if page_count % 5 == 0 {
                    page_count += 1;
                    current_row_index += 1;
                    buttons.push(vec![button]);
                } else {
                    page_count += 1;
                    buttons[current_row_index - 1].push(button);
                }
            })
    } else {
        return Err(GetButtonsError::new(user_doesnt_has_stickers_yet()));
    };

    Ok(page_count)
}
