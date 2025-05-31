use sqlx::prelude::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct SetDeletedColByShortName<'a> {
    short_name: &'a str,
    deleted: bool,
}

impl<'a> SetDeletedColByShortName<'a> {
    pub const fn new(short_name: &'a str, deleted: bool) -> Self {
        Self {
            short_name,
            deleted,
        }
    }

    pub const fn short_name(&self) -> &'a str {
        self.short_name
    }

    pub const fn deleted(&self) -> bool {
        self.deleted
    }
}
