use std::sync::Arc;

use crate::{backend::OutputItem, *};
use serenity::all::{
    Context, CreateAllowedMentions, CreateAttachment, CreateMessage, Embed, Emoji, Http, Message,
};
use std::fmt::Write;
use std::sync::LazyLock;
use tracing::{debug, error, info, instrument, trace};

pub const MAX_MSG_LEN: usize = 1850;

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
- r / run: format and run the code, showing the source, stdout and final stack
- s / show: like run, but only display stdout (or the stack if there is no stdout)
- e / emojify: converts the given code to discord emoji as best as possible

Examples:
- w!fmt below+ 1 2 3
- w! fmt below+ 1 2 3
- w!pad below+ 1 2 3
- w!run below+ 1 2 3
- w!docs tup

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
        Ok((stdout, result)) => {
            let there_is_stdout = !stdout.is_empty();
            let out_is_one_stdout = stdout.len() == 1 && result.is_empty();
            if there_is_stdout {
                output.push_str("---Stdout:\n");
            }
            let (output_stdout, mut attach_stdout) = stdout.into_iter().fold(
                (String::new(), Vec::new()),
                |(mut o_acc, attachments), item| match item {
                    OutputItem::String(s) => {
                        let _ = writeln!(o_acc, "{}", s);
                        (o_acc, attachments)
                    }
                    OutputItem::Svg(s) => update_stdout_output(
                        o_acc,
                        attachments,
                        s.as_bytes(),
                        None,
                        "svg",
                        "svg",
                        out_is_one_stdout,
                    ),
                    OutputItem::Image(bytes, label) => update_stdout_output(
                        o_acc,
                        attachments,
                        &bytes,
                        label,
                        "image",
                        "png",
                        out_is_one_stdout,
                    ),
                    OutputItem::Gif(bytes, label) => update_stdout_output(
                        o_acc,
                        attachments,
                        &bytes,
                        label,
                        "gif",
                        "gif",
                        out_is_one_stdout,
                    ),
                    OutputItem::Audio(bytes, label) => update_stdout_output(
                        o_acc,
                        attachments,
                        &bytes,
                        label,
                        "audio",
                        "ogg",
                        out_is_one_stdout,
                    ),
                    OutputItem::Continuation(n) => {
                        let _ = writeln!(o_acc, "<{n} more item{}>", if n == 1 { "" } else { "s" });
                        (o_acc, attachments)
                    }
                    _ => {
                        let _ = writeln!(o_acc, "<Unimplemented type>",);
                        (o_acc, attachments)
                    }
                },
            );
            output.push_str(&output_stdout);
            attachments.append(&mut attach_stdout);
            if there_is_stdout {
                output.push_str("---End of stdout\n");
            }

            let (output_stack, mut attach_stack) = result.into_iter().fold(
                (String::new(), Vec::new()),
                |(mut output, attachments), item| match item {
                    OutputItem::String(s) => {
                        let _ = writeln!(output, "{}", s);
                        (output, attachments)
                    }
                    OutputItem::Svg(s) => update_stdout_output(
                        output,
                        attachments,
                        s.as_bytes(),
                        None,
                        "svg",
                        "svg",
                        out_is_one_stdout,
                    ),
                    OutputItem::Image(bytes, label) => update_stdout_output(
                        output,
                        attachments,
                        &bytes,
                        label,
                        "image",
                        "png",
                        out_is_one_stdout,
                    ),
                    OutputItem::Gif(bytes, label) => update_stdout_output(
                        output,
                        attachments,
                        &bytes,
                        label,
                        "gif",
                        "gif",
                        out_is_one_stdout,
                    ),
                    OutputItem::Audio(bytes, label) => update_stdout_output(
                        output,
                        attachments,
                        &bytes,
                        label,
                        "audio",
                        "ogg",
                        out_is_one_stdout,
                    ),
                    OutputItem::Continuation(n) => {
                        let _ =
                            writeln!(output, "<{n} more item{}>", if n == 1 { "" } else { "s" });
                        (output, attachments)
                    }
                    _ => {
                        let _ = writeln!(output, "<Unimplemented type>",);
                        (output, attachments)
                    }
                },
            );
            output.push_str(&output_stack);
            attachments.append(&mut attach_stack);
        }
        Err(err) => output = err,
    };

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
    let code = code.trim();
    let code = strip_triple_ticks(code);

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
    let result = run_uiua(strip_triple_ticks(code.trim()));

    let mut output = String::new();
    let mut attachments = Vec::new();
    match result {
        Ok((stdout, result)) => {
            let there_is_stdout = !stdout.is_empty();
            let out_is_one_stdout = stdout.len() == 1 && result.is_empty();
            if there_is_stdout {
                let (output_stdout, mut attach_stdout) = stdout.into_iter().fold(
                    (String::new(), Vec::new()),
                    |(mut o_acc, attachments), item| match item {
                        OutputItem::String(s) => {
                            let _ = writeln!(o_acc, "{}", s);
                            (o_acc, attachments)
                        }
                        OutputItem::Svg(s) => update_stdout_output(
                            o_acc,
                            attachments,
                            s.as_bytes(),
                            None,
                            "svg",
                            "svg",
                            out_is_one_stdout,
                        ),
                        OutputItem::Image(bytes, label) => update_stdout_output(
                            o_acc,
                            attachments,
                            &bytes,
                            label,
                            "image",
                            "png",
                            out_is_one_stdout,
                        ),
                        OutputItem::Gif(bytes, label) => update_stdout_output(
                            o_acc,
                            attachments,
                            &bytes,
                            label,
                            "gif",
                            "gif",
                            out_is_one_stdout,
                        ),
                        OutputItem::Audio(bytes, label) => update_stdout_output(
                            o_acc,
                            attachments,
                            &bytes,
                            label,
                            "audio",
                            "ogg",
                            out_is_one_stdout,
                        ),
                        OutputItem::Continuation(n) => {
                            let _ =
                                writeln!(o_acc, "<{n} more item{}>", if n == 1 { "" } else { "s" });
                            (o_acc, attachments)
                        }
                        _ => {
                            let _ = writeln!(o_acc, "<Unimplemented type>",);
                            (o_acc, attachments)
                        }
                    },
                );
                output.push_str(&output_stdout);
                attachments.append(&mut attach_stdout);
            } else {
                let (output_stack, mut attach_stack) = result.into_iter().fold(
                    (String::new(), Vec::new()),
                    |(mut output, attachments), item| match item {
                        OutputItem::String(s) => {
                            let _ = writeln!(output, "{}", s);
                            (output, attachments)
                        }
                        OutputItem::Svg(s) => update_stdout_output(
                            output,
                            attachments,
                            s.as_bytes(),
                            None,
                            "svg",
                            "svg",
                            out_is_one_stdout,
                        ),
                        OutputItem::Image(bytes, label) => update_stdout_output(
                            output,
                            attachments,
                            &bytes,
                            label,
                            "image",
                            "png",
                            out_is_one_stdout,
                        ),
                        OutputItem::Gif(bytes, label) => update_stdout_output(
                            output,
                            attachments,
                            &bytes,
                            label,
                            "gif",
                            "gif",
                            out_is_one_stdout,
                        ),
                        OutputItem::Audio(bytes, label) => update_stdout_output(
                            output,
                            attachments,
                            &bytes,
                            label,
                            "audio",
                            "ogg",
                            out_is_one_stdout,
                        ),
                        OutputItem::Continuation(n) => {
                            let _ = writeln!(
                                output,
                                "<{n} more item{}>",
                                if n == 1 { "" } else { "s" }
                            );
                            (output, attachments)
                        }
                        _ => {
                            let _ = writeln!(output, "<Unimplemented type>",);
                            (output, attachments)
                        }
                    },
                );
                output.push_str(&output_stack);
                attachments.append(&mut attach_stack);
            }
        }
        Err(err) => output = err,
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
            &format!("Functions don't have more than {} chars", *MAX_FN_LEN),
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

fn strip_triple_ticks(mut s: &str) -> &str {
    s = s.trim();
    s = s.strip_prefix("```").unwrap_or(s);
    s = s.strip_prefix("uiua").unwrap_or(s);
    s = s.strip_suffix("```").unwrap_or(s);
    s
}

fn update_stdout_output(
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
