use std::sync::Arc;

use crate::*;
use serenity::all::{
    Context, CreateAllowedMentions, CreateAttachment, CreateMessage, Embed, Emoji, GuildId, Http,
    Message, ReactionType,
};
use std::fmt::Write;
use std::sync::LazyLock;
use tracing::{debug, error, info, instrument, trace};

pub const MAX_MSG_LEN: usize = 1850;

const HELP_MESSAGE: &str = r#"# wawa
Your friendly neighbourhood uiua bot!

Call upon it with either `w!` or `W!`.

You can delete any wawa message (that you triggered, or whose original message was deleted) by reacting with :x:.
You can get the pad link of any wawa message by reaction with :grey_question: to wawa's response.

Attachments in your message (or the message you're replying to, as well as that message's text) are available as bindings with the following names:
- `I,{N}`: Attachments in the original message
- `R,{N}`: Attachments in the referenced message
- `S`: The text in the referenced message
- Otherwise, the original name will be used
For example, typing `w!r abs S` will uppercase the replied message's text, or error with `Missing binding` if the message isn't a reply.
(Note that they will also be included in the internal (ephemeral) filesystem with their original names: typing `w!r not &fras "somename"` will attempt to negate the contents of the attachment called "somename" (both in your message and the referenced one).
    
Available commands:
- [`ping`]: pong
- [`h` `help`]: display this text!
- [`v` `ver` `version`]: display uiua version used by the rest of commands
- [`f` `fmt` `format`]: run the formatter
- [`p` `pad`]: format and generate a link to the pad
- [`d` `docs`]: show the first paragraph or so of the specified function
- [`r` `run`]: format and run the code, showing the source, stdout and final stack
- [`s` `show`]: like run, but only display stdout (or the stack if there is no stdout)
- [`e` `emojify`]: converts the given code to discord emoji as best as possible

Examples:

- `w!fmt below+ 1 2 3`
- `w! fmt below+ 1 2 3`
- `w!pad below+ 1 2 3`
- `w!run below+ 1 2 3`
- `w!docs tup`

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

pub async fn get_emojis(guild_id: Option<GuildId>, http: &Arc<Http>) -> Vec<Emoji> {
    match guild_id {
        Some(id) => id.emojis(&http).await.ok(),
        None => None,
    }
    .unwrap_or_default()
}

pub fn find_emoji(emojis: &[Emoji], name: &str) -> Option<String> {
    emojis
        .iter()
        .find(|emoji| emoji.name.replace("~1", "") == name.replace(" ", ""))
        .map(|emoji| emoji.to_string())
}

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
    let Some((output, attachments)) = get_output(msg.clone(), http.clone(), code).await else {
        return;
    };
    let source = highlight_code(code.trim());

    // Prepare output
    let result = if output.contains("```") {
        info!(?output, "Output contained triple backticks, denying");
        "Output contained triple backticks, which I disallow".to_string()
    } else if output.is_empty() && attachments.is_empty() {
        trace!("Resulting stack was empty");
        "<Empty stack>".to_string()
    } else if output.is_empty() {
        String::new()
    } else {
        trace!(
            ?code,
            ?output,
            "Sending correctly formed result of running the code"
        );
        format!("```\n{output}\n```")
    };

    // Make sure we're not over the char limit
    let finalized_text = format!("{source}\n{result}");
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
            send_message(
                msg,
                &http,
                "Attempted to send a message that is way too long",
            )
            .await;
        }
    }
}

#[instrument(skip(msg, http))]
pub async fn handle_show(msg: Message, http: Arc<Http>, code: &str) {
    trace!(user = msg.author.name, ?code, "Running show handler");

    let Some((output, attachments)) = get_output(msg.clone(), http.clone(), code).await else {
        return;
    };
    // Prepare output
    let result = if output.contains("```") {
        info!(?output, "Output contained triple backticks, denying");
        "Output contained triple backticks, which I disallow".to_string()
    } else if output.is_empty() && attachments.is_empty() {
        trace!("Result was empty");
        "<Empty result>".to_string()
    } else if output.is_empty() {
        String::new()
    } else {
        trace!(
            ?code,
            ?output,
            "Sending correctly formed result of running the code"
        );
        format!("```\n{output}\n```")
    };
    let finalized_text = result;
    if finalized_text.len() > MAX_MSG_LEN {
        debug!(flen = finalized_text.len(), text = ?&finalized_text.chars().take(200).collect::<String>(), "Final message AND shortened verion were too long");
        send_message(
            msg,
            &http,
            "Attempted to send a message that is way too long",
        )
        .await;
    } else {
        debug!(flen = finalized_text.len(), text = ?&finalized_text.chars().take(200).collect::<String>(), "Showing normally, size is correct");
        send_message_advanced(
            msg,
            &http,
            CreateMessage::new()
                .content(finalized_text)
                .add_files(attachments),
        )
        .await;
    }
}
#[instrument(skip(msg, ctx))]
pub async fn handle_docs(msg: Message, ctx: Context, code: &str) {
    trace!(user = msg.author.name, ?code, "Running docs handler");
    if code.len() > *MAX_FN_LEN {
        debug!("Code was too long to show documentation");
        send_message(
            msg,
            &ctx.http,
            &format!(
                "There's no function with more than {} chars, silly",
                *MAX_FN_LEN
            ),
        )
        .await
    } else {
        trace!(?code, "Sending back documentation");
        send_message(
            msg.clone(),
            &ctx.http.clone(),
            &get_docs(code.trim(), ctx, msg).await,
        )
        .await;
    }
}
#[instrument(skip(msg, ctx))]
pub async fn handle_emojification(msg: Message, ctx: Context, code: &str) {
    let emojificated = emojificate(code, msg.clone(), ctx.clone()).await;
    send_message(msg, &ctx.http, &emojificated).await;
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

pub fn strip_triple_ticks(mut s: &str) -> &str {
    s = s.trim();
    s = s.strip_prefix("```").unwrap_or(s);
    s = s.strip_prefix("\n").unwrap_or(s);
    s = s.strip_prefix("uiua").unwrap_or(s);

    s = s.strip_suffix("\n").unwrap_or(s);
    s = s.strip_suffix("```").unwrap_or(s);
    s
}

pub fn update_stdout_output(
    mut output: String,
    mut attachments: Vec<CreateAttachment>,
    bytes: &[u8],
    label: Option<String>,
    name: &str,
    ext: &str,
    should_add_attachment_text: bool,
) -> (String, Vec<CreateAttachment>) {
    if should_add_attachment_text {
        if let Some(l) = label {
            let _ = writeln!(
                output,
                "<attachment #{}: {name} '{l}'>",
                attachments.len() + 1
            );
        } else {
            let _ = writeln!(output, "<attachment #{}: {name}>", attachments.len() + 1);
        }
    }
    attachments.push(CreateAttachment::bytes(
        bytes,
        format!("{name}_{}.{ext}", attachments.len() + 1),
    ));
    (output, attachments)
}

pub fn strip_wawa_prefix(text: &str) -> Option<String> {
    let prefixes = [
        "w!",
        "W!",
        //"Wawa!", // these intersect with the toki pona usage of the word!
        //"wawa!",
        &format!("@{}", *SELF_HANDLE),
        &format!("<@{}>", *SELF_ID),
        &format!("<@&{}>", *SELF_ID), /* Self-role */
    ];

    let lines = text
        .lines()
        .skip_while(|line| {
            !prefixes
                .iter()
                .any(|prefix| line.trim_start().starts_with(prefix))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let lines = lines.trim_start();

    prefixes
        .iter()
        .fold(None, |acc, prefix| {
            acc.or_else(|| lines.strip_prefix(prefix))
        })
        .map(|s| s.to_string())
}

pub fn is_question_mark(c: &ReactionType) -> bool {
    c == &ReactionType::Unicode('❔'.into())
}
