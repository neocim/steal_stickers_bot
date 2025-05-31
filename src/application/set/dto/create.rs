use sqlx::prelude::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct Create<'a> {
    tg_id: i64,
    short_name: &'a str,
    title: &'a str,
}

impl<'a> Create<'a> {
    pub const fn new(tg_id: i64, short_name: &'a str, title: &'a str) -> Self {
        Self {
            tg_id,
            short_name,
            title,
        }
    }

    pub const fn tg_id(&self) -> i64 {
        self.tg_id
    }

    pub const fn short_name(&self) -> &'a str {
        self.short_name
    }

    pub const fn title(&self) -> &'a str {
        self.title
    }
}
