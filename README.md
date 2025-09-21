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

## Multi-kennel rewrite
Tables:

kennels (
    id PRIMARY KEY
    name UNIQUE
    guild_id
    role_id UNIQUE
    msg_announce?
    msg_announce_edit?
    msg_release?
    kennel_channel?
    kennel_msg?
    kennel_msg_edit?
    kennel_release_msg?
    CONSTRAINT edits
)

kennelings (
    id PRIMARY KEY
    kennel_id REFERENCES kennels
    guild_id REFERENCES kennels.guild_id
    author_id
    victim_id
    kenneled_at DEFAULT CURRENT_TIMESTAMP
    kennel_length
    released_at GENERATED ALWAYS AS ((kenneled_at + kennel_length)) STORED
    msg_announce_id?
    kennel_msg_id?
)

What happens when someone _deletes_ a kennel?
- Should be moved to an archived table?
- Enabled flag on kennels?
- Don't allow this behavior?
- Drop all previous kennels?
- I think, we should move all kennels into an archived_kennels table for statistics tracking.


start up bot:
  look at all kennelings since bot was last shut down
  update everything:
    - should role still be applied?
    - should messages be edited?
        - are the original handles still there?
  for active ones:
    - spawn new task that will do that garbage in a minute
  (these two behaviors are pretty similar, methinks! we could probably consolidate these?)

wildcard:
  look at all kennel commands from that guild
  any match?
  send some messages
  lock some people up

config updates:
  - changing channel: nothing should happen

commands:
    - /config add_kennel <kennel_name> <role_id>
    - /config set_{msg_announce,msg_announce_edit,msg_release,kennel_msg,kennel_msg_edit,kennel_release_msg} <kennel_name> <val?>
    