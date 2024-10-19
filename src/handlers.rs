use std::sync::Arc;

use crate::*;
use serenity::all::{CreateAllowedMentions, CreateAttachment, CreateMessage, Embed, Http, Message};
use std::sync::LazyLock;
use tracing::{debug, error, info, instrument, trace};

const MAX_MSG_LEN: usize = 1700;

const HELP_MESSAGE: &str = r#"# wawa
Your friendly neighbourhood uiua bot!

Run with either `w!` or `wawa!`

Available commands:
- ping: pong
- h / help: display this text!
- v / ver / version: display uiua version used by the rest of commands
- f / fmt / format: run the formatter
- p / pad: format and generate a link to the pad
- d / docs <fn>: show the first paragraph or so of the specified function
- r / run: format and run the code
- e / emojify: converts the given code to discord emoji as best as possible

Examples:
- w!fmt below+ 1 2 3
- w! fmt below+ 1 2 3
- w!pad below+ 1 2 3
- w!run below+ 1 2 3
- w!docs tup

(Do note that many IO operations are blocked, which includes `&p`, `&fras`, etc.)

Ping <@328851809357791232> for any questions or if you want the version to get bumped
"#;

static MAX_FN_LEN: LazyLock<usize> = LazyLock::new(|| {
    uiua::PrimClass::all()
        .flat_map(|pc| pc.primitives())
        .map(|p| {
            p.names()
                .text
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .len()
        })
        .max()
        .unwrap() // There _are_ primitives
});

/// The color are: pink, red, yellow, green, teal, blue
const OUTPUT_COLOR_CYCLE: [u8; 6] = [34, 36, 32, 33, 31, 35];

// HANDLERS
#[instrument(skip_all)]
pub async fn handle_ping(msg: Message, http: Arc<Http>) {
    trace!("Running ping handler");
    send_message(msg, &http, "Pong!").await
}

#[instrument(skip_all)]
pub async fn handle_version(msg: Message, http: Arc<Http>) {
    trace!("Running version handler");
    send_message(msg, &http, uiua::VERSION).await
}

#[instrument(skip_all)]
pub async fn handle_help(msg: Message, http: Arc<Http>) {
    trace!("Running help handler");
    send_message(msg, &http, HELP_MESSAGE).await
}

#[instrument(skip(msg, http))]
pub async fn handle_fmt(msg: Message, http: Arc<Http>, code: &str) {
    trace!(user = msg.author.name, ?code, "Running fmt handler");
    send_message(msg, &http, &highlight_code(strip_triple_ticks(code.trim()))).await
}

#[instrument(skip(msg, http))]
pub async fn handle_pad(msg: Message, http: Arc<Http>, code: &str) {
    trace!(user = msg.author.name, ?code, "Running pad handler");
    send_message(msg, &http, &format_and_get_pad_link(code.trim())).await;
}

#[instrument(skip(msg, http))]
pub async fn handle_run(msg: Message, http: Arc<Http>, code: &str) {
    trace!(user = msg.author.name, ?code, "Running run handler");
    let code = code.trim();
    let code = strip_triple_ticks(code);
    // TODO: strip single ticks as well
    //let code = strip_single_ticks(code);

    if code.contains("```") {
        info!(code = %code, "Input contained backticks, disallowing");
        send_message(
            msg,
            &http,
            "Input contained triple backticks, which I disallow",
        )
        .await;
        return;
    }

    let source = highlight_code(code.trim());
    let result = run_uiua(strip_triple_ticks(code.trim()));

    let mut output = String::new();
    let mut attachments = Vec::new();
    match result {
        Ok(result) => {
            let stack_len = result.len();
            for (i, item) in result.into_iter().enumerate() {
                match item {
                    OutputItem::Audio(bytes) => {
                        output
                            .push_str(&format!("<attachment #{}: audio>\n", attachments.len() + 1));
                        attachments.push(CreateAttachment::bytes(
                            bytes,
                            format!("audio_{}.ogg", attachments.len() + 1),
                        ));
                    }
                    OutputItem::Image(bytes) => {
                        output
                            .push_str(&format!("<attachment #{}: image>\n", attachments.len() + 1));
                        attachments.push(CreateAttachment::bytes(
                            bytes,
                            format!("image_{}.png", attachments.len() + 1),
                        ));
                    }
                    OutputItem::Gif(bytes) => {
                        output
                            .push_str(&format!("<attachment #{}: image>\n", attachments.len() + 1));
                        attachments.push(CreateAttachment::bytes(
                            bytes,
                            format!("image_{}.gif", attachments.len() + 1),
                        ));
                    }
                    OutputItem::Misc(val) => {
                        if stack_len > 1 {
                            output.push_str(&format!(
                                "\x1b[{}m{}\x1b[0m",
                                OUTPUT_COLOR_CYCLE[i % OUTPUT_COLOR_CYCLE.len()],
                                val.show()
                            ))
                        } else {
                            output.push_str(val.show().as_str())
                        };
                        output.push('\n');
                    }
                    OutputItem::Continuation(more) => {
                        output.push_str(&format!(
                            "<{more} more item{}>\n",
                            if more == 1 { "" } else { "s" }
                        ));
                    }
                }
            }
        }
        Err(err) => output = err,
    };

    let result = if output.contains("```") {
        info!(?output, "Output contained triple backticks, denying");
        "Output contained triple backticks, which I disallow".to_string()
    } else if output.is_empty() {
        trace!("Resulting stack was empty");
        "<Empty stack>".to_string()
    } else {
        trace!(
            ?code,
            ?output,
            "Sending correctly formed result of running the code"
        );
        format!("```ansi\n{output}\n```")
    };

    let finalized_text = format!("Source:\n{source}\nReturns:\n{result}");
    let shortened_text =
        format!("<Resulting message is too large, skipping the source>\nReturns:\n{result}");

    match (finalized_text.len(), shortened_text.len()) {
        (f, _) if f < MAX_MSG_LEN => {
            debug!(flen = f, text = ?&finalized_text.chars().take(200).collect::<String>(), "Sending full-length version");
            send_message_advanced(
                msg,
                &http,
                CreateMessage::new()
                    .content(finalized_text)
                    .add_files(attachments),
            )
            .await;
        }
        (f, s) if f > MAX_MSG_LEN && s <= MAX_MSG_LEN => {
            debug!(flen = f, slen = s, text = ?&finalized_text.chars().take(200).collect::<String>(), shortened = ?&shortened_text.chars().take(300).collect::<String>(), "Final message was too long, sending shortened version");
            send_message_advanced(
                msg,
                &http,
                CreateMessage::new()
                    .content(shortened_text)
                    .add_files(attachments),
            )
            .await;
        }
        (f, s) => {
            debug!(flen = f, slen = s, text = ?&finalized_text.chars().take(200).collect::<String>(), "Final message AND shortened verion were too long");
            send_message(msg, &http, "Message is way too long").await;
        }
    }
}
#[instrument(skip(msg, http))]
pub async fn handle_docs(msg: Message, http: Arc<Http>, code: &str) {
    trace!(user = msg.author.name, ?code, "Running docs handler");
    if code.len() > *MAX_FN_LEN {
        debug!("Code was too long to show documentation");
        send_message(
            msg,
            &http,
            &format!("Functions don't have more than {} chars", *MAX_FN_LEN),
        )
        .await
    } else {
        trace!(?code, "Sending back documentation");
        send_message(msg, &http, &get_docs(code.trim())).await
    }
}
#[instrument(skip(msg, http))]
pub async fn handle_emojification(msg: Message, http: Arc<Http>, code: &str) {
    send_message(msg, &http, "Emojification hasn't been completed yet").await;
}

#[instrument(skip(msg, http))]
pub async fn handle_unrecognized(msg: Message, http: Arc<Http>, code: &str) {
    trace!(
        user = msg.author.name,
        ?code,
        "Handling unrecognized command"
    );
    let unrec = code.trim();
    let shortened = unrec.chars().take(10).collect::<String>();
    trace!("Someone sent an unrecognized command: '{shortened}'");
    send_message(
        msg,
        &http,
        &format!("I don't recognize '{}' as a command :pensive:", shortened),
    )
    .await;
}

// HELPERS

#[instrument(skip_all)]
pub async fn send_message(msg: Message, http: &Arc<Http>, mut text: &str) {
    info!(user = ?msg.author.name, text, "Sending message");
    if text.len() > MAX_MSG_LEN {
        text = "Attempted to send a message that is way too long";
    }
    match msg.reply(http, text).await {
        Ok(_) => {}
        Err(e) => error!(reason = ?e, user = msg.author.name, "Error while sending"),
    };
}

#[instrument(skip_all)]
pub async fn send_embed(msg: Message, http: &Arc<Http>, mut text: &str, embed: Embed) {
    info!(user = ?msg.author.name, text, "Sending message that contains embed");
    if text.len() > MAX_MSG_LEN {
        text = "Message is way too long";
        send_message(msg, http, text).await;
        return;
    }
    let builder = CreateMessage::new()
        .content(text)
        .embed(embed.into())
        .reference_message(&msg)
        .allowed_mentions(CreateAllowedMentions::default() /* Nobody */);

    match msg.channel_id.send_message(http, builder).await {
        Ok(_) => {}
        Err(e) => error!(
            reason = ?e,
            user = msg.author.name,
            "Error while sending with embed"
        ),
    };
}

// TODO rename
#[instrument(skip_all)]
pub async fn send_message_advanced(msg: Message, http: &Arc<Http>, builder: CreateMessage) {
    trace!("Building up advanced message");
    let builder = builder
        .reference_message(&msg)
        .allowed_mentions(CreateAllowedMentions::new().replied_user(false));
    match msg.channel_id.send_message(http, builder).await {
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
