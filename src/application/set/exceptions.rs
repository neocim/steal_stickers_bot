use std::borrow::Cow;

use crate::application::common::exceptions::ApplicationException;

#[derive(Debug, thiserror::Error)]
#[error("sticker set with short name `{short_name}` already exists: {message}")]
pub struct SetShortNameAlreadyExist<'a> {
    short_name: &'a str,
    message: Cow<'static, str>,
}

impl<'a> SetShortNameAlreadyExist<'a> {
    pub fn new(short_name: &'a str, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            short_name,
            message: message.into(),
        }
    }
}

impl<'a> ApplicationException for SetShortNameAlreadyExist<'a> {}

#[derive(Debug, thiserror::Error)]
#[error("sticker set with short name `{short_name}` not exists: {message}")]
pub struct SetShortNameNotExist<'a> {
    short_name: &'a str,
    message: Cow<'static, str>,
}

impl<'a> SetShortNameNotExist<'a> {
    pub fn new(short_name: &'a str, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            short_name,
            message: message.into(),
        }
    }
}

impl<'a> ApplicationException for SetShortNameNotExist<'a> {}

#[derive(Debug, thiserror::Error)]
#[error("sticker sets with Telegram ID `{tg_id}` not exists: {message}")]
pub struct SetTgIdNotExist {
    tg_id: i64,
    message: Cow<'static, str>,
}

impl SetTgIdNotExist {
    pub fn new(tg_id: i64, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            tg_id,
            message: message.into(),
        }
    }
}

impl ApplicationException for SetTgIdNotExist {}
