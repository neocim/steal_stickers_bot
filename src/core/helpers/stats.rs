pub struct GlobalStats {
    pub total_stolen: i64,
    pub total_users: usize,
}

impl GlobalStats {
    pub const fn new(total_stolen: i64, total_users: usize) -> Self {
        Self {
            total_stolen,
            total_users,
        }
    }
}

pub struct PersonalStats {
    pub total_user_sets_count: i64,
    pub not_deleted_user_sets_count: i64,
}

impl PersonalStats {
    pub const fn new(total_user_sets_count: i64, not_deleted_user_sets_count: i64) -> Self {
        Self {
            total_user_sets_count,
            not_deleted_user_sets_count,
        }
    }
}
