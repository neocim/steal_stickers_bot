use std::borrow::Cow;

#[derive(Clone)]
pub enum GetOwnerState {
    GetStickers,
}

impl GetOwnerState {
    const fn as_str(&self) -> &'static str {
        match self {
            GetOwnerState::GetStickers => "get_stickers",
        }
    }
}

impl From<GetOwnerState> for Cow<'static, str> {
    fn from(state: GetOwnerState) -> Self {
        Cow::Borrowed(state.as_str())
    }
}

impl PartialEq<&str> for GetOwnerState {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}
