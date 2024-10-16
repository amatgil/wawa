use std::sync::Arc;

use crate::*;
use serenity::all::{Http, Message};

const HELP_MESSAGE: &str = r#"The help message has not been written yet!"#;

// HANDLERS
pub async fn handle_ping(msg: Message, http: Arc<Http>) {
    send_message(msg, &http, "Pong!").await
}
pub async fn handle_version(msg: Message, http: Arc<Http>) {
    send_message(msg, &http, uiua::VERSION).await
}

pub async fn handle_help(msg: Message, http: Arc<Http>) {
    send_message(msg, &http, HELP_MESSAGE).await
}
pub async fn handle_highlight(msg: Message, http: Arc<Http>, code: &str) {
    send_message(msg, &http, &highlight_code(strip_triple_ticks(code.trim()))).await
}
pub async fn handle_pad(msg: Message, http: Arc<Http>, code: &str) {
    send_message(msg, &http, &format_and_get_pad_link(code.trim())).await;
}
pub async fn handle_run(msg: Message, http: Arc<Http>, code: &str) {
    let source = highlight_code(code.trim());
    let result = run_uiua(strip_triple_ticks(code.trim()));
    let finalized = format!("Source:\n{source}\nReturns:\n{result}");
    send_message(msg, &http, &finalized).await
}
pub async fn handle_docs(msg: Message, http: Arc<Http>, code: &str) {
    send_message(msg, &http, &get_docs(code.trim())).await
}
pub async fn handle_unrecognized(msg: Message, http: Arc<Http>, code: &str) {
    let unrec = code.trim();
    let shortened = &unrec[0..(30.min(unrec.len()))];
    eprintln!("Someone sent an unrecognized command: '{shortened}'");
    send_message(msg, &http, &format!("I don't recognize '{}' as a command :pensive:", shortened)).await;
}

// HELPERS

async fn send_message(msg: Message, http: &Arc<Http>, mut text: &str) {
    if text.len() > 1000 {
        text = "Message is way too long";
    }
    match msg.reply(http, text).await {
        Ok(_) => {}
        Err(e) => eprintln!("Error sending message: {e}"),
    };
}

fn strip_triple_ticks(mut s: &str) -> &str {
    s = s.trim();
    s = s.strip_prefix("```").unwrap_or(s);
    s = s.strip_prefix("uiua").unwrap_or(s);
    s = s.strip_suffix("```").unwrap_or(s);
    s
}
