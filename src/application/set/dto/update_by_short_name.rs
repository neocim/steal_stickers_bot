use sqlx::prelude::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct UpdateSetByShortName<'a> {
    short_name: &'a str,
    title: Option<&'a str>,
    deleted: Option<bool>,
}

impl<'a> UpdateSetByShortName<'a> {
    pub const fn new(short_name: &'a str, title: Option<&'a str>, deleted: Option<bool>bool) -> Self {
        Self {
            short_name,
            title,
            deleted,
        }
    }

    pub const fn short_name(&self) -> &'a str {
        self.short_name
    }

    pub const fn title(&self) -> Option<&'a str> {
        self.title
    }

    pub const fn deleted(&self) -> Option<bool> {
        self.deleted
    }
}
