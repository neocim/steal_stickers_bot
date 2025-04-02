use std::sync::Arc;

use async_trait::async_trait;
use chrono::{NaiveTime, Utc};
use grammers_client::Client as ClientGrammers;
use telers::{
    FromContext, Request,
    errors::{EventErrorKind, MiddlewareError},
    event::EventReturn,
    middlewares::{OuterMiddleware, outer::MiddlewareResponse},
};
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

#[derive(Debug, Clone)]
pub struct ClientApplicationMiddleware {
    pub key: &'static str,
    pub client: Arc<ClientGrammers>,
    pub last_update_time: Arc<NaiveTime>,
    pub api_id: i32,
    pub api_hash: String,
}

impl ClientApplicationMiddleware {
    pub fn new(client: ClientGrammers, api_id: i32, api_hash: String) -> Self {
        Self {
            key: "client",
            client: Arc::new(client),
            last_update_time: Arc::new(Utc::now().time()),
            api_id,
            api_hash,
        }
    }
}

#[async_trait]
impl OuterMiddleware for ClientApplicationMiddleware {
    async fn call(&mut self, mut request: Request) -> Result<MiddlewareResponse, EventErrorKind> {
        let now = Utc::now().time();

        if (now - *self.last_update_time).num_minutes() >= 10 {
            debug!("Update client");
            self.last_update_time = Arc::new(now);

            let client = client_connect(self.api_id, self.api_hash.clone())
                .await
                .map_err(MiddlewareError::new)?;
            self.client = Arc::new(client.clone());

            request.context.insert(self.key, client);
        } else {
            let client = (*self.client).clone();

            request.context.insert(self.key, client);
        }

        Ok((request, EventReturn::default()))
    }
}
