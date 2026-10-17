use CallbackDataPrefix::*;

pub enum CallbackDataPrefix {
    MyStickers,
    Stats,
    StealSet,
}

impl CallbackDataPrefix {
    pub const fn as_str(&self) -> &str {
        match self {
            MyStickers => "MyStickers",
            Stats => "Stats",
            StealSet => "StealSet",
        }
    }
}

impl AsRef<str> for CallbackDataPrefix {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
