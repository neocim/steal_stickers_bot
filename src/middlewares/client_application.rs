use tokio::sync::Mutex;

use telers::{
    errors::{EventErrorKind, MiddlewareError},
    event::EventReturn,
    middlewares::{outer::MiddlewareResponse, OuterMiddleware},
    FromContext, Request,
};

use async_trait::async_trait;
use chrono::{NaiveTime, Utc};
use grammers_client::Client as ClientGrammers;

use tracing::debug;

use crate::telegram_application::client_connect;

#[derive(Debug, Clone, FromContext)]
#[context(key = "client", from = ClientGrammers)]
pub struct Client(pub ClientGrammers);

impl From<ClientGrammers> for Client {
    fn from(value: ClientGrammers) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct ClientApplicationMiddleware {
    pub key: &'static str,
    pub client: Mutex<ClientGrammers>,
    pub last_update_time: Mutex<NaiveTime>,
    pub api_id: i32,
    pub api_hash: String,
}

impl ClientApplicationMiddleware {
    pub fn new(client: ClientGrammers, api_id: i32, api_hash: String) -> Self {
        Self {
            key: "client",
            client: Mutex::new(client),
            last_update_time: Mutex::new(Utc::now().time()),
            api_id,
            api_hash,
        }
    }
}

#[async_trait]
impl OuterMiddleware for ClientApplicationMiddleware {
    async fn call(&self, mut request: Request) -> Result<MiddlewareResponse, EventErrorKind> {
        let mut lock = self.last_update_time.lock().await;

        let now = Utc::now().time();

        if (now - *lock).num_minutes() >= 10 {
            debug!("Update client");

            *lock = now;

            let client = client_connect(self.api_id, self.api_hash.clone())
                .await
                .map_err(MiddlewareError::new)?;

            let mut lock = self.client.lock().await;
            *lock = client.clone();

            request.context.insert(self.key, client);
        } else {
            let client = (*self.client.lock().await).clone();

            request.context.insert(self.key, client);
        }

        Ok((request, EventReturn::default()))
    }
}
