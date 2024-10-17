pub use std::sync::Arc;

use dotenv;
use serenity::{
    all::{Http, Ready}, async_trait, futures::future::BoxFuture, model::channel::Message, prelude::*
};
use wawa::*;

use std::{collections::HashMap, fmt::Debug, future::Future, ops::{Add, Deref}};
const SELF_ID: &str = "<@1295816766446108795>";
const SELF_ROLE: &str = "<@&1295816766446108795>";

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let contents = msg.content_safe(ctx.cache).clone();
        let trimmed = contents.trim();
        dbg!(&trimmed);

        let commanded = trimmed.strip_prefix("w!")
            .or_else(||trimmed.strip_prefix("wawa!"))
            .or_else(||trimmed.strip_prefix(&format!("{SELF_ID}"))
            .or_else(||trimmed.strip_prefix(&format!("{SELF_ROLE}"))));

        if let Some(s) = commanded {
            let s = s.strip_prefix(" ").unwrap_or_else(|| s);
            let space_idx = s.bytes().position(|c| c == b' ').unwrap_or_else(||s.len());
            match &s[0..space_idx] {
                "ping" => handle_ping(msg, ctx.http).await,
                "ver" | "version" => handle_version(msg, ctx.http).await,
                "help" => handle_help(msg, ctx.http).await,
                "fmt" => handle_fmt(msg, ctx.http, &s[space_idx..].trim()).await,
                "pad" => handle_pad(msg, ctx.http, &s[space_idx..].trim()).await,
                "docs" => handle_docs(msg, ctx.http, &s[space_idx..].trim()).await,
                "run" => handle_run(msg, ctx.http, &s[space_idx..].trim()).await,
                unrec => handle_unrecognized(msg, ctx.http, unrec).await,
            }
        } else { // We're not a command, but we can check if the message contains an un-markdown'd link
            eprintln!("Checkign for pad link");
            /*
            if has_raw_pad_link(&msg.content) {
                eprintln!("FOUND PAD LINK");
                let link = "<link go here>";
                let response = format!("You seem to have sent a raw pad link. Please use markdown links next time (like [this](<link>) next time). For now, here is [the link you sent]({link})");
                send_message(msg, &ctx.http, &response).await;
            }
            */
        }


    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected", ready.user.name)
    }
}

#[tokio::main]
async fn main() {
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
