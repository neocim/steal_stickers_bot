use std::time::Duration;

use grammers_client::FixedReconnect;

pub const SESSION_FILE: &str = "configs/user.session";
pub const RECONNECT_POLICY: FixedReconnect = FixedReconnect {
    attempts: 3,
    delay: Duration::from_millis(100),
};
