pub use std::sync::Arc;
use std::sync::LazyLock;

use serenity::{
    all::{ArgumentConvert, Emoji, Ready},
    async_trait,
    model::channel::Message,
    prelude::*,
};
use tracing::{debug, error, info, instrument, span, trace, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::{writer::MakeWriterExt, SubscriberBuilder},
    EnvFilter,
};
use wawa::*;

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
    trace!(text = trimmed, "Starting to parse message");

    let commanded = trimmed
        .strip_prefix("w!")
        .or_else(|| trimmed.strip_prefix("wawa!"))
        .or_else(|| trimmed.strip_prefix(&format!("@{}", *SELF_HANDLE)))
        .or_else(|| trimmed.strip_prefix(&format!("<@{}>", *SELF_ID)))
        .or_else(|| trimmed.strip_prefix(&format!("<@&{}>", *SELF_ID /* Self-role */)));

    if let Some(s) = commanded {
        let span = span!(Level::TRACE, "command_handler");
        let _guard = span.enter();
        info!(user = msg.author.name, body = ?s, "Processing body");

        let s = s.trim();
        let space_idx = s
            .bytes()
            .position(|c| c.is_ascii_whitespace())
            .unwrap_or(s.len());
        debug!(cmd = s[0..space_idx].trim(), "Parsing command");

        match s[0..space_idx].trim() {
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

    async fn ready(&self, _: Context, ready: Ready) {
        info!(name = ready.user.name, "Bot is connected");
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
    let intents = GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILD_MESSAGES;

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
