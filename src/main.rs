use std::collections::HashMap;

use ::serenity::all::{
    Builder, CommandInteraction, CommandOption, Context as SerenityCtx, CreateCommand,
    CreateCommandOption, Event, EventHandler, FullEvent, Interaction, InteractionCreateEvent,
    ResolvedValue, UserId,
};
use dotenv::dotenv;
use poise::{ApplicationContext, Command, FrameworkContext, serenity_prelude as serenity};
use regex::Regex;
use sqlx::{PgPool, postgres::PgPoolOptions};

// User data, which is stored and accessible in all command invocations
struct Data {
    pool: PgPool,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Displays your or another user's account creation date
#[poise::command(slash_command)]
async fn set_kennel_role(
    ctx: Context<'_>,
    #[description = "The kenneling role"] role: serenity::Role,
) -> Result<(), Error> {
    let Data { pool } = ctx.data();

    sqlx::query(
        r#"
        INSERT INTO servers(guild_id, role_id) 
        VALUES ($1, $2)
        ON CONFLICT (guild_id)
        DO UPDATE SET
            role_id=$2
            ;
    "#,
    )
    .bind(format!("{}", role.guild_id.get()))
    .bind(format!("{}", role.id.get()))
    .execute(pool)
    .await?;

    ctx.reply(format!(
        "Successfully set this guild's kennel role to <@&{}>",
        role.id
    ))
    .await?;

    Ok(())
}

#[poise::command(slash_command)]
async fn set_kennel_command(
    ctx: Context<'_>,
    #[description = "The command to kennel someone"] command: String,
) -> Result<(), Error> {
    let Data { pool } = ctx.data();

    let guild_id = ctx.guild_id().expect("No guild ID???");

    let re = Regex::new(r"^[a-zA-Z]+$").unwrap();

    if !re.is_match(command.as_str()) {
        ctx.reply(format!(
            "Cannot set the command to {}! Make sure it only uses letters!",
            &command
        ))
        .await?;

        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO servers(guild_id, command_name) 
        VALUES ($1, $2)
        ON CONFLICT (guild_id)
        DO UPDATE SET
            command_name=$2
            ;
    "#,
    )
    .bind(format!("{}", &guild_id))
    .bind(&command)
    .execute(pool)
    .await?;

    let cmd = CreateCommand::new(&command)
        .description("Punish a user!")
        .add_option(
            CreateCommandOption::new(
                serenity::CommandOptionType::User,
                "user",
                "User to be punished",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                serenity::CommandOptionType::String,
                "time",
                "How long to punish the user",
            )
            .required(true),
        );

    guild_id.create_command(ctx, cmd).await?;

    ctx.reply(format!(
        "Successfully set this guild's kennel command to: /{}",
        &command
    ))
    .await?;

    Ok(())
}

#[poise::command(slash_command)]
async fn set_kennel_message(
    ctx: Context<'_>,
    #[description = "The message to send when kenneling someone"] message: String,
) -> Result<(), Error> {
    todo!()
}

#[poise::command(slash_command)]
async fn kennel_user(
    ctx: Context<'_>,
    #[description = "User to kennel"] user: UserId,
    #[description = "Time to kennel"] time: String,
) -> Result<(), Error> {
    let data = sqlx::query!(
        r#"
            SELECT * FROM servers
            WHERE
                guild_id = $1
                AND command_name = $2
                ;
        "#,
        format!("{}", ctx.guild_id().ok_or("No guild ID!")?.get()),
        ctx.invoked_command_name()
    )
    .fetch_one(&ctx.data().pool)
    .await?;

    ctx.reply(format!(
        "
Command Name: {}
Command Verb: {}
Guild ID: {}
Kennel Role: {}
Command ID: {}
",
        data.command_name.unwrap_or("none".into()),
        data.command_verb.unwrap_or("none".into()),
        data.guild_id,
        data.role_id.unwrap_or("none".into()),
        data.command_id.unwrap_or("none".into()),
    ))
    .await?;

    Ok(())
}

async fn wildcard_command_handler(
    ctx: &SerenityCtx,
    event: &FullEvent,
    framework_ctx: FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    println!("firing (handler");
    if let FullEvent::InteractionCreate { interaction } = event {
        println!("firing (interact create");
        if let Interaction::Command(command_interaction) = interaction {
            println!("firing (command)");
            if command_interaction.data.name == "set_kennel_command"
                || command_interaction.data.name == "set_kennel_role"
            {
                return Ok(());
            }

            // Apparently you can just... do this???? It feels so wrong.
            // No options validation here. I didn't even bother to find the
            // command - everything will use kennel_user().
            let app_ctx = ApplicationContext {
                data,
                serenity_context: ctx,
                interaction: command_interaction,
                interaction_type: poise::CommandInteractionType::Command,
                args: &command_interaction.data.options(),
                has_sent_initial_response: &std::sync::atomic::AtomicBool::new(false),
                framework: framework_ctx,
                parent_commands: &vec![],
                command: &kennel_user(),
                invocation_data: &tokio::sync::Mutex::new(Box::new(()) as _),
                __non_exhaustive: (),
            };

            let action = app_ctx
                .command
                .slash_action
                .ok_or("command structure mismatch")?;

            let _ = action(app_ctx).await;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = std::env::var("BOT_TOKEN").expect("missing BOT_TOKEN");
    let postgres_url = std::env::var("DATABASE_URL").expect("missing DATABASE_URL");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![set_kennel_role(), set_kennel_command()],
            event_handler: |w, x, y, z| Box::pin(wildcard_command_handler(w, x, y, z)),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let pool = PgPoolOptions::new()
                    .max_connections(5)
                    .connect(postgres_url.as_str())
                    .await?;
                Ok(Data { pool })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
