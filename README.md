<h1 align="center">steal_stickers_bot</h1>
<div align="center">
        <h4><a href="https://t.me/steal_stickers_bot">bot in Telegram</a>
</div>

<h2>Running The Bot</h2>
<h3>Preparing</h3>

1. Install [rustup](https://www.rust-lang.org/tools/install), [justfile](https://github.com/casey/just?tab=readme-ov-file#pre-built-binaries), [sqlx-cli](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#install).
2. Install [Docker](https://docs.docker.com/get-docker/) and [Docker Compose](https://docs.docker.com/compose/install/).
3. Create your Telegram application [following instructions](https://core.telegram.org/api/obtaining_api_id).
4. Create a new bot with [@BotFather](https://t.me/BotFather).
5. Clone this repository and change directory:
```
git clone https://github.com/neocim/steal_stickers_bot
cd steal_stickers_bot/
```
6. Optionally run tests, using: 
```
cargo test
```
7. Copy [config.toml.example](./configs/config.toml.example), remove `.example` from name of file and fill it required information.
8. Copy [.env.example](./.env.example), remove `.example` from name of file and fill it ***the same*** required information as in your file `config.toml`.

<h3>Running</h3>

1. First, we need to pull actual image of the bot, so as not to build it ourselves:
```
just pull-img
```
> You can also build it manually using `just build`

2. Then we need to authorize the client. Do it, using:
```
just auth
```
> A code should be sent to your Telegram account. Enter it into the terminal without any extra characters.

3. To finally run the bot, use:
```
just compose-run
```

4. After the previous step, bot will start working, but database will be without migrations. To solve it, run the command below (it uses information from [.env](./.env.example) file):
```
just migrate
```

<strong>If you encounter errors that are directly related to my code (docker errors, bot bugs, etc.), please [open an Issue](https://github.com/neocim/steal_stickers_bot/issues/new). Thanks :)</strong>

<h2>License</h2>

Licensed under:
- MIT License ([LICENSE](./LICENSE) or https://opensource.org/license/MIT)
