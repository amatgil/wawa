use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::backend::{NativisedWebBackend, OutputItem};
use crate::*;
use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use serenity::all::{Attachment, Context, CreateAttachment, Emoji, Http, Message};
use std::fmt::Write;
use std::str;
use tracing::{info, trace};
use uiua::{PrimDoc, SysBackend};
use uiua::{PrimDocFragment, PrimDocLine, Primitive, Uiua};

const MIN_AUTO_IMAGE_DIM: usize = 30;
const MAX_STACK_VALS_DISPLAYED: usize = 10;
const DEFAULT_EXECUTION_LIMIT: Duration = Duration::from_secs(5);

impl From<uiua::Value> for OutputItem {
    fn from(value: uiua::Value) -> Self {
        fn try_from_ogg(value: &Value) -> Result<OutputItem, Box<dyn std::error::Error>> {
            let channels: Vec<Vec<f32>> = value_to_audio_channels(value)?
                .into_iter()
                .map(|v| v.into_iter().map(|x| x as f32).collect())
                .collect();
            let mut sink = Vec::new();
            let mut encoder = vorbis_rs::VorbisEncoderBuilder::new(
                std::num::NonZeroU32::new(44100).ok_or("unreachable")?,
                std::num::NonZeroU8::new(channels.len() as u8).ok_or("unreachable")?,
                &mut sink,
            )?
            .build()?;
            encoder.encode_audio_block(channels)?;
            encoder.finish()?;
            Ok(OutputItem::Audio(sink, None))
        }
        use uiua::media::*;
        use uiua::Value;

        // Audio?
        if value.shape.last().is_some_and(|&n| n >= 44100 / 4)
            && matches!(&value, Value::Num(arr) if arr.elements().all(|x| x.abs() <= 5.0))
        {
            if let Ok(this) = try_from_ogg(&value) {
                trace!("Turning audio into bytes");
                return this;
            }
        }
        // Image?
        if let Ok(image) = value_to_image(&value) {
            if image.width() >= MIN_AUTO_IMAGE_DIM as u32
                && image.height() >= MIN_AUTO_IMAGE_DIM as u32
            {
                if let Ok(bytes) = image_to_bytes(&image, image::ImageFormat::Png) {
                    trace!("Turning image into bytes");
                    return OutputItem::Image(bytes, None);
                }
            }
        }
        // Gif
        if let Ok(gif) = value_to_gif_bytes(&value, 16.0) {
            match &*value.shape {
                &[f, h, w] | &[f, h, w, _]
                    if h >= MIN_AUTO_IMAGE_DIM && w >= MIN_AUTO_IMAGE_DIM && f >= 5 =>
                {
                    trace!("Turning gif into bytes");
                    return OutputItem::Gif(gif, None);
                }
                _ => {}
            }
        }

        OutputItem::String(value.show())
    }
}

/// Returns (stdout, top-most elements of stack)
pub async fn run_uiua(
    code: &str,
    attachments: &[Attachment],
    // The text of the message that this command is in reply to
    text_of_refd: Option<&str>,
    // Attachments of the message that this command is in reply to
    attachments_of_refd: Option<&[Attachment]>,
) -> Result<(Vec<OutputItem>, Vec<OutputItem>), String> {
    const MAX_ATTACHMENT_IMAGE_PIXEL_COUNT: u32 = 2048 * 2048;

    trace!(code, "Starting to execute uiua code");
    if code.is_empty() {
        return Err("Cannot run empty code".into());
    }

    let backend = NativisedWebBackend::default();
    let mut full_code = String::new();

    let push_attachments = async |attchs: &[Attachment],
                                  acc: &mut String,
                                  bindname_if_img: &str| {
        let mut i = 0; // image index, not incremented for non-image attachemnts

        for attachment in attchs.iter().rev() {
            let url = &attachment.url;
            let filename = attachment.filename.clone();
            let data = if let (Some(w), Some(h)) = (attachment.width, attachment.height) {
                if w * h > MAX_ATTACHMENT_IMAGE_PIXEL_COUNT {
                    return Err(format!(
                               "Attachment {i} has (width, height) := ({w}, {h}), which \
                                is too many pixels ({}) (maximum is {MAX_ATTACHMENT_IMAGE_PIXEL_COUNT})",
                               w*h));
                }
                acc.push_str(&format!(
                    "{} = popunimg&frab\"{filename}\"\n",
                    format!("{bindname_if_img}__{i}")
                ));
                i += 1;
                reqwest::get(url)
                    .await
                    .map_err(|_| format!("could not get image data associated with {filename}'"))?
            } else {
                reqwest::get(url)
                    .await
                    .map_err(|_| format!("could not get attachment data for '{filename}'"))?
            };

            backend
                .file_write_all(
                    filename.as_ref(),
                    &data.bytes().await.map_err(|_| {
                        format!("could not interpret bytes of attachment {filename}'")
                    })?,
                )
                .unwrap();
        }
        Ok(())
    };
    if let Some(text) = text_of_refd {
        // This is so scuffed, there's definitely a proper way to make a proper binding from Rust
        let text_split = text
            .lines()
            .map(|l| format!("$ {l} \n"))
            .collect::<String>();
        full_code.push_str("\n");
        full_code.push_str(&text_split);
        full_code.push_str("\n");
        full_code.push_str("S =\n");
        backend.file_write_all(&Path::new("S"), &text.as_bytes())?;
    }
    push_attachments(attachments, &mut full_code, "I").await?;
    if let Some(a_refd) = attachments_of_refd {
        push_attachments(a_refd, &mut full_code, "R").await?;
    }

    full_code.push_str(&format!("{code}\n"));

    let mut runtime = Uiua::with_backend(backend).with_execution_limit(DEFAULT_EXECUTION_LIMIT);

    match runtime.compile_run(|comp| comp.experimental(true).load_str(&full_code)) {
        Ok(_c) => {
            drop(_c); // Holds ref to backend, cringe
            trace!(code, "Code ran successfully");
            let stack = runtime.take_stack();
            let stack_len = stack.len();
            let stdout: Vec<_> = runtime
                .take_backend::<NativisedWebBackend>()
                .unwrap()
                .current_stdout()
                .to_vec();

            Ok((
                stdout,
                stack
                    .into_iter()
                    .take(MAX_STACK_VALS_DISPLAYED)
                    .map(|val| val.into())
                    .chain((stack_len > MAX_STACK_VALS_DISPLAYED).then(|| {
                        OutputItem::Continuation((stack_len - MAX_STACK_VALS_DISPLAYED) as u32)
                    }))
                    .collect(),
            ))
        }
        Err(e) => {
            trace!(code, "Code ran Unsuccessfully");
            Err(format!("Error while running: {e} "))
        }
    }
}

pub async fn get_docs(f: &str, ctx: Context, msg: Message) -> String {
    let emojis = get_emojis(msg.guild_id, &ctx.http).await;

    if f.trim().is_empty() {
        return "Documentation is [here](https://uiua.org/docs/)".to_string();
    }

    match Primitive::from_format_name(f)
        .or_else(|| Primitive::from_glyph(f.chars().next().unwrap_or_default()))
        .or_else(|| Primitive::from_name(f))
    {
        Some(docs) => {
            let short = PrimDoc::from(docs)
                .short
                .iter()
                .map(|frag| format!("## {}", print_doc_frag(&emojis, frag)))
                .collect::<Vec<_>>()
                .join("\n");

            let name = docs.name();
            let name = name.split_once(" ").map(|pair| pair.0).unwrap_or(name);
            let final_result = |long: &str| {
                format!(
                    "\n{short}\n\n\n{long}\n\n([More information](https://uiua.org/docs/{name}))"
                )
            };

            let long =
                PrimDoc::from(docs)
                    .lines
                    .into_iter()
                    .fold(String::new(), |mut acc, docs| {
                        let new = print_docs(&emojis, &docs);
                        if final_result(&acc).len() + new.len() + 1 < MAX_MSG_LEN {
                            acc.push('\n');
                            acc.push_str(&new);
                        }
                        acc
                    });

            final_result(&long)
        }
        None => format!(
            "No docs found for '{f}', did you spell it right? (For full docs, see [full docs](https://www.uiua.org/docs))"
        ),
    }
}

fn print_doc_frag(emojis: &[Emoji], frag: &PrimDocFragment) -> String {
    match frag {
        PrimDocFragment::Text(t) => t.clone(),
        PrimDocFragment::Code(t) => format!("`{t}`"),
        PrimDocFragment::Emphasis(t) => format!("_{t}_"),
        PrimDocFragment::Strong(t) => format!("**{t}**"),
        PrimDocFragment::Primitive { prim, .. } => {
            find_emoji(emojis, prim.name()).unwrap_or_else(|| format!("`{prim}`"))
        }
        PrimDocFragment::Link { text, url } => format!("[{text}]({url})"),
    }
}

fn print_docs(emojis: &[Emoji], line: &PrimDocLine) -> String {
    match line {
        PrimDocLine::Text(vs) => vs
            .iter()
            .map(|frag| print_doc_frag(emojis, frag))
            .collect::<Vec<_>>()
            .join(" "),
        PrimDocLine::Example(e) => {
            let out = match e.output().as_ref().map(|vs| vs.join(";")) {
                Ok(l) => l,
                Err(l) => l.to_string(),
            };
            let text = format!(
                "{}\n{}",
                e.input(),
                out.lines().fold(String::new(), |mut out, l| {
                    let _ = writeln!(out, "# {l}");
                    out
                })
            );

            highlight_code(&text).to_string()
        }
    }
}

pub fn format_and_get_pad_link(code: &str) -> String {
    const THE_LINK: &str = "<https://www.youtube.com/watch?v=dQw4w9WgXcQ>";

    let encoded = URL_SAFE.encode(code);
    let link = if code.trim().is_empty() {
        THE_LINK.to_string()
    } else {
        format!("https://www.uiua.org/pad?src={}__{encoded}", uiua::VERSION)
    };

    let result = format!("[pad]({link}) for: {}", highlight_code(&code));
    let shortened = format!("[pad]({link})");

    if result.len() <= MAX_MSG_LEN {
        trace!("Sending pad message normally");
        result
    } else if shortened.len() <= MAX_MSG_LEN {
        trace!("Pad message was too long, skipping source");
        shortened
    } else {
        trace!("Pad message was too long, period");
        "The resulting link does not fit in a discord message! :(".to_string()
    }
}

pub async fn get_output(
    msg: Message,
    http: Arc<Http>,
    code: &str,
) -> Option<(String, Vec<CreateAttachment>)> {
    let code = strip_triple_ticks(code.trim());

    if code.contains("```") {
        info!(code = %code, "Input contained backticks, disallowing");
        send_message(
            msg,
            &http,
            "Input contained triple backticks, which I disallow",
        )
        .await;
        return None;
    }
    let text_of_refd = msg
        .referenced_message
        .as_ref()
        .map(|refd| refd.content.clone());
    let attachments_of_refd = msg.referenced_message.map(|refd| refd.attachments);
    let result = run_uiua(
        strip_triple_ticks(code.trim()),
        &msg.attachments,
        text_of_refd.as_deref(),
        attachments_of_refd.as_deref(),
    );

    let mut output = String::new();
    let mut attachments = Vec::new();
    match result.await {
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

    Some((output, attachments))
}
