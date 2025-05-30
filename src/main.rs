pub mod application;
mod config;
mod core;
mod domain;
mod infrastructure;
mod launch;
mod presentation;

use launch::launch;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    launch().await;
}
