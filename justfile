docker-build:
    docker build -t steal_stickers_bot .

auth:
    docker run -it --rm \
        --mount type=bind,source=./configs,target=/app/configs \
        --name steal_stickers_bot nnenty/steal_stickers_bot:latest \
        auth

run:
    docker run --rm \
        --log-driver local --log-opt max-size=100m \
        --mount type=bind,source=./configs,target=/app/configs \
        --name steal_stickers_bot nnenty/steal_stickers_bot:latest \
        run

compose-run:
    docker compose up && \

compose-build:
    docker compose up --build

migrate-run username=env("POSTGRES_USER") \
            password=env("POSTGRES_PASSWORD") \
            host=env("POSTGRES_HOST") \
            port=env("POSTGRES_PORT") \
            db=env("POSTGRES_DB"):
    sqlx migrate run \
    --source ./src/infrastructure/database/migrations \
    --database-url="postgres://{{username}}:{{password}}@{{host}}:{{port}}/{{db}}"

# This is just my template to run databse without `just compose-run`. If you want to override it, then use this template:
# docker run --rm --name {NAME} -p {PORT}:5432 -e POSTGRES_PASSWORD={PASSWORD} -e POSTGRES_USER={USER} -e POSTGRES_DB={DATABASE_NAME} postgres:17-alpine
run-db:
    docker run --rm --name steal_stickers_bot_db \
        -p 5432:5432 \
        -e POSTGRES_USER=admin \
        -e POSTGRES_PASSWORD=123 \
        -e POSTGRES_DB=db \
        postgres:17-alpine
