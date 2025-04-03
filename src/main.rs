pub mod application;
mod commands;
pub mod config;
pub mod core;
pub mod domain;
pub mod infrastructure;
mod launch;
pub mod middlewares;
pub mod router;
mod telegram_application;

use crate::launch::launch;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    launch().await;
}
