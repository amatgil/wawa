pub use std::sync::Arc;

use dotenv;
use serenity::{
    all::{Http, Ready}, async_trait, futures::future::BoxFuture, model::channel::Message, prelude::*
};
use wawa::*;

use std::{collections::HashMap, fmt::Debug, future::Future, ops::{Add, Deref}};
const SELF_ID: &str = "<@1295816766446108795>";

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        //let commands: HashMap<Vec<&str>, &dyn Command> = HashMap::from([
        //    (vec!["ping"], &handle_ping as &dyn Command),
        //    (vec!["version"], &handle_version as &dyn Command),
        //]);

        let contents = msg.content.clone();
        let s = contents.trim();
        let Some(s) = s.strip_prefix("w!")
            .or_else(||s.strip_prefix("wawa!"))
            .or_else(||s.strip_prefix("{SELF_ID}")) else { return };

        let space_idx = s.bytes().position(|c| c == b' ').unwrap_or_else(||s.len());

        match &s[0..space_idx] {
            "ping" => handle_ping(msg, ctx.http).await,
            "ver" | "version" => handle_version(msg, ctx.http).await,
            "help" => handle_help(msg, ctx.http).await,
            "high" | "highlight" => handle_highlight(msg, ctx.http, &s[space_idx..].trim()).await,
            "pad" => handle_pad(msg, ctx.http, &s[space_idx..].trim()).await,
            "docs" => handle_docs(msg, ctx.http, &s[space_idx..].trim()).await,
            "run" => handle_run(msg, ctx.http, &s[space_idx..].trim()).await,
            unrec => handle_unrecognized(msg, ctx.http, unrec).await,
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

fn is_command<'a, 'b>(m: &'a str, cmd: &'b str) -> Option<&'a str> {
    m.strip_prefix(&format!("wawa!{}", cmd))
        .or_else(|| m.strip_prefix(&format!("w!{}", cmd)))
        .or_else(|| m.strip_prefix(&format!("{SELF_ID}{}", cmd))).map(|s| s.trim())
}


fn strip_triple_ticks(mut s: &str) -> &str {
    s = s.trim();
    s = s.strip_prefix("```").unwrap_or(s);
    s = s.strip_prefix("uiua").unwrap_or(s);
    s = s.strip_suffix("```").unwrap_or(s);
    s
}
