use serde::{Deserialize, Serialize};
use serenity::{
    all::{Reaction, ReactionType, Ready},
    async_trait,
    model::channel::Message,
    prelude::*,
};
pub use std::sync::Arc;
use std::{
    fs::{self, read_to_string, File},
    io::{Read, Write},
    path::PathBuf,
    sync::LazyLock,
};
use tracing::{debug, error, info, instrument, span, trace, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::{writer::MakeWriterExt, SubscriberBuilder},
    EnvFilter,
};
use wawa::*;

const ANSWERED_MESSAGES_PATH: &str = "/tmp/wawa_answered_messages_path/";

static SELF_HANDLE: LazyLock<String> =
    LazyLock::new(|| dotenv::var("BOT_SELF_HANDLE").unwrap_or_else(|_| "wawa#0280".into()));
static SELF_ID: LazyLock<u64> =
    LazyLock::new(|| match dotenv::var("BOT_SELF_ID").map(|str| str.parse()) {
        Ok(Ok(id)) => id,
        _ => 1295816766446108795,
    });

struct Handler;

#[instrument(skip_all)]
async fn handle_message(ctx: Context, msg: Message) {
    if msg.author.bot {
        return;
    }
    let contents = msg.content_safe(ctx.cache.clone()).clone();
    let trimmed = contents.trim();
    trace!(
        text = trimmed,
        srv = msg
            .guild_id
            .map(|g| g.to_string())
            .unwrap_or("No GuildID".to_string()),
        "Starting to parse message"
    );

    let prefixes = [
        "w!",
        "W!",
        "Wawa!",
        "wawa!",
        &format!("@{}", *SELF_HANDLE),
        &format!("<@{}>", *SELF_ID),
        &format!("<@&{}>", *SELF_ID), /* Self-role */
    ];

    let lines = trimmed
        .lines()
        .skip_while(|line| {
            !prefixes
                .iter()
                .any(|prefix| line.trim_start().starts_with(prefix))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let lines = lines.trim_start();

    let commanded = prefixes.iter().fold(None, |acc, prefix| {
        acc.or_else(|| lines.strip_prefix(prefix))
    });

    if let Some(s) = commanded {
        let span = span!(Level::TRACE, "command_handler");
        let _guard = span.enter();
        info!(user = msg.author.name, body = ?s, "Processing body");

        info!("saving as pending to answeredness file");
        let answered_path = PathBuf::from(ANSWERED_MESSAGES_PATH);
        let mut random_hash: u128 = rand::random();
        while let Ok(true) = fs::exists(answered_path.join(random_hash.to_string())) {
            random_hash = rand::random();
        }
        let answered_lock_file_path = answered_path.join(random_hash.to_string());
        match fs::File::create(&answered_lock_file_path) {
            Err(e) => error!(?e, "could not create file in answeredness direcory"),
            Ok(mut f) => {
                _ = f.write(serenity::json::to_string(&msg).unwrap().as_bytes());
            }
        };

        let s = s.trim();
        let space_idx = s
            .bytes()
            .position(|c| c.is_ascii_whitespace())
            .unwrap_or(s.len());
        debug!(cmd = s[0..space_idx].trim(), "Parsing command");

        match s[0..space_idx].trim().to_lowercase().as_str() {
            "ping" => handle_ping(msg, ctx.http).await,
            "v" | "ver" | "version" => handle_version(msg, ctx.http).await,
            "h" | "help" | "" => handle_help(msg, ctx.http).await,
            "f" | "fmt" | "format" => handle_fmt(msg, ctx.http, s[space_idx..].trim()).await,
            "p" | "pad" => handle_pad(msg, ctx.http, s[space_idx..].trim()).await,
            "d" | "doc" | "docs" | "what" => handle_docs(msg, ctx, s[space_idx..].trim()).await,
            "e" | "emojify" => handle_emojification(msg, ctx, s[space_idx..].trim()).await,
            "r" | "run" => handle_run(msg, ctx.http, s[space_idx..].trim()).await,
            "s" | "show" => handle_show(msg, ctx.http, s[space_idx..].trim()).await,
            "shutdown" => send_message(msg, &ctx.http, "Ok, shutting down now").await, // This does not shutdown
            unrec => handle_unrecognized(msg, ctx.http, unrec).await,
        }

        match fs::remove_file(&answered_lock_file_path) {
            Ok(()) => info!(
                "answering complete, deleting file '{}'",
                answered_lock_file_path.display()
            ),
            Err(e) => error!(
                ?e,
                "failed to remove answeredness file at '{}'",
                answered_lock_file_path.display()
            ),
        }
    } else {
        let span = span!(Level::TRACE, "rulethree_handler");
        let _guard = span.enter();

        trace!(user = msg.author.name, "Checking for pad link");

        let vs = extract_raw_pad_link(trimmed);
        if !vs.is_empty() {
            trace!(
                user = msg.author.name,
                link = vs[0],
                "Link without markdown detected"
            );
            let link = &vs[0];
            info!(author = ?msg.author, "Found a pad link");
            let response = format!("You've sent a raw pad link! Please use markdown links next time (like `[this](<link>)`). For now, here is [the link you sent]({link})");
            send_message(msg, &ctx.http, &response).await;
        } else {
            trace!(user = msg.author.name, "No pad link detected");
        }
        std::mem::drop(_guard);
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        tokio::spawn(handle_message(ctx, msg));
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(name = ready.user.name, "Bot is connected");
        trace!("deleting global commands, just in case");
        serenity::model::application::Command::set_global_commands(&ctx.http, vec![])
            .await
            .unwrap();
        trace!("deleted global commands");

        // Respond to all messages that failed
        info!("Responding to all messages that wawa started to respond to, but didn't finalize");

        _ = fs::create_dir(ANSWERED_MESSAGES_PATH);
        let paths = fs::read_dir(ANSWERED_MESSAGES_PATH).unwrap();
        for path in paths {
            // SAFETY: if something happens between opening the dir and reading the path,
            // we want to panic anyway
            let path = path.as_ref().unwrap().path();
            match read_to_string(&path) {
                Err(e) => error!(?e, "could not read from answered file"),
                Ok(contents) => {
                    trace!(path = ?path, "read file with ok");
                    let msg: Message = match serde_json::from_str(&contents) {
                        Ok(m) => m,
                        Err(e) => {
                            error!(?path, ?e, "could not read json message from path");
                            continue;
                        }
                    };
                    send_message(msg, &ctx.http,
                                 "Wawa seems to have been shutdown during the computation you summoned! It's likely it triggered the Out-Of-Memory-Killer; please try to not send messages that use up that much memory.").await;
                    info!(?path, "responded with OOM to path");
                    _ = fs::remove_file(path);
                }
            }
        }
    }
    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        let reacted_message = match reaction.message(&ctx.http).await {
            Ok(rm) => rm,
            Err(e) => {
                trace!(?e, "Received emoji but couldn't obtain its source message");
                return;
            }
        };

        let command_message = match reacted_message.clone().referenced_message {
            Some(cm) => cm,
            None => {
                trace!("Wawa message seems to be in reply to an unreachable message"); // Probs deleted
                                                                                       // We will accept it regardless
                match reacted_message.delete(ctx.http).await {
                    Ok(()) => trace!("Message deleted (command does not exist anymore, so we're accepting deletion command)"),
                    Err(error) => trace!(?error, "Error deleting message"),
                }
                return;
            }
        };

        if Some(command_message.author.id) == reaction.user_id {
            // Authorized user sent it!
            trace!(
                user = command_message.author.name,
                "Authorized emoji detected on wawa message"
            );

            if reaction.emoji != ReactionType::Unicode("❌".to_string()) {
                trace!("Emoji is NOT cross, skipping");
                return;
            }

            trace!("Emoji is cross, proceeding to deletion");
            match reacted_message.delete(ctx.http).await {
                Ok(()) => trace!("Message deleted"),
                Err(error) => trace!(?error, "Error deleting message"),
            }
        } else {
            // Unauthorized, this is all for tracing
            let emoji_sender: Option<String> = if let Some(emoji_sender_id) = reaction.user_id {
                ctx.http
                    .get_user(emoji_sender_id)
                    .await
                    .map(|r| r.name)
                    .ok()
            } else {
                None
            };
            trace!(
                command_sender = command_message.author.name,
                emoji_sender,
                "UN-authoritzed emoji detected on wawa message, ignoring"
            );
        }
    }
}

#[tokio::main]
async fn main() {
    let logs_dir = dotenv::var("LOGS_DIRECTORY").expect("'LOGS_DIRECTORY not found in .env file");
    let file_appender = RollingFileAppender::new(Rotation::DAILY, logs_dir, "wawa_log");
    let subscriber = SubscriberBuilder::default()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stdout.and(file_appender))
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    let token = dotenv::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not found in .env");
    let intents = GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    info!("Starting up wawa");

    // Create a new instance of the Client, logging in as a bot.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    trace!("Client started");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        error!("Client error: {why:?}");
    }
}
