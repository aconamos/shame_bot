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
- [ ] statistics
- [ ] separate logic for kennel to respond and then set a timer for a new callback fn that does more verification (user still exists? didn't leave? (if left, pause duration)? same kennel channel? what messages got sent? which should be sent?)
- [ ] proper errors
- [ ] rewire the wildcard handler? don't use poise commands, and just use context?
- [ ] clean up all those little messy TODOs
- [ ] consolidate the SQL queries in healthcheck
- [ ] toss every command error in the trash?