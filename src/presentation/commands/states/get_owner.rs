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

impl AsRef<str> for GetOwnerState {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<&str> for GetOwnerState {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}
