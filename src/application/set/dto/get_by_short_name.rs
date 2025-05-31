use sqlx::prelude::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct GetByShortName<'a> {
    short_name: &'a str,
}

impl<'a> GetByShortName<'a> {
    pub const fn new(short_name: &'a str) -> Self {
        Self { short_name }
    }

    pub const fn short_name(&self) -> &'a str {
        self.short_name
    }
}
