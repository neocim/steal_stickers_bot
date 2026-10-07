use std::{io, sync::Arc};

use grammers_client::tl::{
    enums::{self, InputStickerSet},
    functions::messages::GetStickerSet,
    types::{self, InputStickerSetShortName},
};
use grammers_client::{Client, SenderPool, SignInError};
use grammers_session::storages::SqliteSession;
use tracing::info;

pub mod constants;
mod errors;

pub async fn client_connect(
    session: Arc<SqliteSession>,
    api_id: i32,
) -> Result<Client, errors::Error> {
    let SenderPool { runner, handle, .. } = SenderPool::new(session, api_id);

    let client = Client::new(handle);
    let _ = tokio::spawn(runner.run());

    return Ok(client);
}

pub async fn client_authorize(
    client: &Client,
    api_hash: &str,
    phone: &str,
    password: &str,
) -> Result<(), errors::Error> {
    if !client.is_authorized().await? {
        let token = client.request_login_code(phone, api_hash).await?;

        println!("Enter the code you received on your Telegram account:");
        let mut code = String::new();
        io::stdin().read_line(&mut code)?;
        let code = code.trim();

        match client.sign_in(&token, code).await {
            Err(SignInError::PasswordRequired(password_token)) => {
                client
                    .check_password(password_token, password.trim())
                    .await?;
            }
            Ok(_) => (),
            Err(err) => return Err(err.into()),
        };
        info!("Signed in!");
    } else {
        info!("Already signed in!")
    }

    Ok(())
}

pub async fn get_sticker_set_user_id(
    set_name: &str,
    client: &Client,
) -> Result<i64, errors::Error> {
    let set_id = match client
        .invoke(&GetStickerSet {
            stickerset: InputStickerSet::ShortName(InputStickerSetShortName {
                short_name: set_name.to_owned(),
            }),
            hash: 0,
        })
        .await?
    {
        enums::messages::StickerSet::Set(types::messages::StickerSet {
            set: enums::StickerSet::Set(types::StickerSet { id, .. }),
            ..
        }) => id,
        _ => todo!(),
    };

    let mut user_id = set_id >> 32;
    if set_id >> 24 & 0xff == 1 {
        user_id += 0x100000000
    }

    Ok(user_id)
}
