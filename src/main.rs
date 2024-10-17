pub use std::sync::Arc;

use dotenv;
use serenity::{
    all::Ready,
    async_trait,
    model::channel::Message,
    prelude::*,
};
use tracing::instrument;
use wawa::*;

const SELF_AT: &str = "@wawa#0280";
const SELF_ID: &str = "<@1295816766446108795>";
const SELF_ROLE: &str = "<@&1295816766446108795>";

struct Handler;


#[instrument ]
async fn handle_message(ctx: Context, msg: Message) {
    if msg.author.bot {
        return;
    }
    let contents = msg.content_safe(ctx.cache).clone();
    let trimmed = contents.trim();
    dbg!(&trimmed);

    let commanded = trimmed.strip_prefix("w!")
        .or_else(|| trimmed.strip_prefix("wawa!"))
        .or_else(|| trimmed.strip_prefix(SELF_ID))
        .or_else(|| trimmed.strip_prefix(SELF_AT))
        .or_else(|| trimmed.strip_prefix(SELF_ROLE));

    if let Some(s) = commanded {
        let s = s.trim();
        dbg!(s);
        let space_idx = s.bytes().position(|c| c == b' ').unwrap_or_else(|| s.len());
        match dbg!(s[0..space_idx].trim()) {
            "ping" => handle_ping(msg, ctx.http).await,
            "v" | "ver" | "version" => handle_version(msg, ctx.http).await,
            "h" | "help" => handle_help(msg, ctx.http).await,
            "f" | "fmt" => handle_fmt(msg, ctx.http, &s[space_idx..].trim()).await,
            "p" | "pad" => handle_pad(msg, ctx.http, &s[space_idx..].trim()).await,
            "d" | "doc" | "docs" => handle_docs(msg, ctx.http, &s[space_idx..].trim()).await,
            "r" | "run" => handle_run(msg, ctx.http, &s[space_idx..].trim()).await,
            unrec => handle_unrecognized(msg, ctx.http, unrec).await,
        }
    } else {
        // We're not a command, but we can check if the message contains an un-markdown'd link
        eprintln!("Checkign for pad link");
        let vs = extract_raw_pad_link(trimmed);
        if !vs.is_empty() {
            let link = &vs[0];
            dbg!("FOUND PAD LINK");
            let response = format!("You've sent a raw pad link! Please use markdown links next time (like [this](<link>)). For now, here is [the link you sent]({link})");
            send_message(msg, &ctx.http, &response).await;
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        tokio::spawn(async move {
            handle_message(ctx, msg).await;
        });
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected", ready.user.name)
    }
}

#[tokio::main]
async fn main() {
    //tracing_subscriber::

    let token = dotenv::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not found in .env");
    // Login with a bot token from the environment
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILD_MESSAGES;

    // Create a new instance of the Client, logging in as a bot.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
