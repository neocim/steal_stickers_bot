use sqlx::prelude::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct DeleteByShortName<'a> {
    short_name: &'a str,
}

impl<'a> DeleteByShortName<'a> {
    pub const fn new(short_name: &'a str) -> Self {
        Self { short_name }
    }

    pub const fn short_name(&self) -> &'a str {
        self.short_name
    }
}
