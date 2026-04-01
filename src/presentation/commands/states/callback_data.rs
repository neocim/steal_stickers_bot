use CallbackDataPrefix::*;

pub enum CallbackDataPrefix {
    MyStickers,
    Stats,
}

impl CallbackDataPrefix {
    pub const fn as_str(&self) -> &str {
        match self {
            MyStickers => "MyStickers",
            Stats => "Stats",
        }
    }
}

impl AsRef<str> for CallbackDataPrefix {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
