use std::sync::Arc;

use crate::*;
use serenity::all::{Http, Message};

const HELP_MESSAGE: &str = r#"# wawa
Your friendly neighbourhood uiua bot!

Run with either `w!` or `wawa!`

Available commands:
- ping: pong
- ver / version: display uiua version used by the rest of commands
- help: display this text!
- fmt: run the formatter
- pad: format and generate a link to the pad
- run: format and run the code
- docs <fn>: show the first paragraph or so of the specified function

Examples:
- w!fmt below+ 1 2 3
- w! fmt below+ 1 2 3
- w!pad below+ 1 2 3
- w!run below+ 1 2 3
- w!docs tup

(Do note that many IO operations are blocked, which includes `&p`, `&fras`, etc.)

Ping <@328851809357791232> for any questions or if you want the version to get bumped
"#;

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
pub async fn handle_fmt(msg: Message, http: Arc<Http>, code: &str) {
    send_message(msg, &http, &highlight_code(strip_triple_ticks(code.trim()))).await
}
pub async fn handle_pad(msg: Message, http: Arc<Http>, code: &str) {
    send_message(msg, &http, &format_and_get_pad_link(code.trim())).await;
}
pub async fn handle_run(msg: Message, http: Arc<Http>, code: &str) {
    let code = code.trim();
    let code = strip_triple_ticks(code);
    // TODO: strip single ticks as well

    let code = strip_single_ticks(code);

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

pub async fn send_message(msg: Message, http: &Arc<Http>, mut text: &str) {
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

fn strip_single_ticks(mut s: &str) -> &str {
    s = s.trim();
    s = s.strip_prefix("`").unwrap_or(s);
    s = s.strip_suffix("`").unwrap_or(s);
    s
}
