pub mod application;
pub mod config;
pub mod core;
pub mod domain;
pub mod infrastructure;
mod launch;
mod presentation;

use crate::launch::launch;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    launch().await;
}
