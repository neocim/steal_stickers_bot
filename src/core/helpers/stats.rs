pub enum GreaterThan {
    FirstLevel,
    SecondLevel,
    ThirdLevel,
    FourthLevel,
}

impl GreaterThan {
    pub const fn as_str(&self) -> &str {
        match self {
            GreaterThan::FirstLevel => "25",
            GreaterThan::SecondLevel => "50",
            GreaterThan::ThirdLevel => "75",
            GreaterThan::FourthLevel => "100",
        }
    }
}

impl Into<i64> for GreaterThan {
    fn into(self) -> i64 {
        match self {
            GreaterThan::FirstLevel => 25,
            GreaterThan::SecondLevel => 50,
            GreaterThan::ThirdLevel => 75,
            GreaterThan::FourthLevel => 100,
        }
    }
}

pub struct GlobalStats {
    pub total_stolen: i64,
    pub first_count: u32,
    pub second_count: u32,
    pub third_count: u32,
    pub fourth_count: u32,
}

impl GlobalStats {
    pub const fn new(
        total_stolen: i64,
        first_count: u32,
        second_count: u32,
        third_count: u32,
        fourth_count: u32,
    ) -> Self {
        Self {
            total_stolen,
            first_count,
            second_count,
            third_count,
            fourth_count,
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
