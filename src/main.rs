mod uiuaizing;

pub use std::sync::Arc;

use dotenv;
use serenity::{
    all::{Http, Ready},
    async_trait,
    model::channel::Message,
    prelude::*,
};
use std::collections::HashMap;
use uiuaizing::{get_docs, highlight_code, run_uiua};

const HELP_MESSAGE: &str = r#"The help message has not been written yet!"#;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let s = msg.content.clone();
        if is_command(&s, "ping").is_some() {
            send_message(msg, &ctx.http, "Pong!").await;
        } else if is_command(&s, "help").is_some() {
            send_message(msg, &ctx.http, HELP_MESSAGE).await;
        } else if let Some(code) = is_command(&s, "run") {
            let result = run_uiua(strip_triple_ticks(code));
            send_message(msg, &ctx.http, &result).await;
        } else if let Some(f) = is_command(&s, "docs") {
            send_message(msg, &ctx.http, &get_docs(f.trim())).await;
        } else if let Some(code) = is_command(&s, "fmt") {
            send_message(msg, &ctx.http, &highlight_code(strip_triple_ticks(code))).await;
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

async fn send_message(msg: Message, http: &Arc<Http>, text: &str) {
    match msg.channel_id.say(http, text).await {
        Ok(_) => {}
        Err(why) => println!("Error sending message: {why:?}"),
    }
}

fn is_command<'a, 'b>(m: &'a str, cmd: &'b str) -> Option<&'a str> {
    m.strip_prefix(&format!("!wawa {}", cmd))
        .or_else(|| m.strip_prefix(&format!("!w {}", cmd)))
}


fn strip_triple_ticks(mut s: &str) -> &str {
    s = s.trim();
    s = s.strip_prefix("```").unwrap_or(s);
    s = s.strip_prefix("uiua").unwrap_or(s);
    s = s.strip_suffix("```").unwrap_or(s);
    s
}
