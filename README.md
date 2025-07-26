# Setup

## Environment variables
- Rename `dot-env` to `.env` and populate the fields

## Run
- If developing, `cargo run` will do
- For production: 
    1. Run `cargo install sqlx-cli`
    2. Run `cargo sqlx prepare`
    3. Run `docker compose up --build -d`

## TODO
- [ ] prevent the bot from breaking when people leave (fix in healthcheck, the kennel command, and set_kennel_role) (also, suspend sentence?)
- [ ] proper tracing/logging
- [ ] statistics