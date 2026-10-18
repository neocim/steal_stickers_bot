use telers::CallbackData;

#[derive(CallbackData, Clone, Debug)]
#[callback_data(prefix = "Stats")]
pub struct StatsState {
    pub state: String,
}

#[derive(CallbackData, Clone, Debug)]
#[callback_data(prefix = "MyStickers")]
pub struct MyStickersPage {
    pub page: usize,
}
