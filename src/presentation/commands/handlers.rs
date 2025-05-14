use std::borrow::Cow;

pub mod add_stickers;
pub mod bot_src;
pub mod cancel;
pub mod deleted_sets_upd;
pub mod my_stickers;
pub mod start;
pub mod steal_pack;

#[derive(Debug, Clone, thiserror::Error)]
#[error("Error occurded while adding stickers: {message}")]
pub(crate) struct AddStickersError {
    message: Cow<'static, str>,
}

impl AddStickersError {
    fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
