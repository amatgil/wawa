use std::sync::Arc;
use std::time::Duration;

use crate::backend::{NativisedWebBackend, OutputItem};
use crate::*;
use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use serenity::all::{
    ArgumentConvert, Attachment, Context, CreateAttachment, Emoji, EmojiParseError, Http, Message,
};
use serenity::futures::future::join_all;
use std::fmt::Write;
use std::str;
use tracing::{info, trace};
use uiua::SysBackend;
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
            match value.shape.dims() {
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
) -> Result<(Vec<OutputItem>, Vec<OutputItem>), String> {
    const MAX_ATTACHMENT_IMAGE_PIXEL_COUNT: u32 = 2048 * 2048;

    trace!(code, "Starting to execute uiua code");
    if code.is_empty() {
        return Err("Cannot run empty code".into());
    }

    let backend = NativisedWebBackend::default();
    let mut full_code = String::new();

    for (i, attachment) in attachments.iter().rev().enumerate() {
        let i = attachments.len() - i - 1;
        let url = &attachment.url;
        match (attachment.width, attachment.height) {
            (Some(w), Some(h)) if w * h > MAX_ATTACHMENT_IMAGE_PIXEL_COUNT => {
                return Err(format!(
                    "Attachment {i} has (width, height) := ({w}, {h}), which is too many pixels (maximum is {MAX_ATTACHMENT_IMAGE_PIXEL_COUNT})"
                ))
            }
            (None, _) | (_, None) => return Err(format!("Attachment {i} did not come with a width or height, and I've only implemented images for now")),
            _ => {}
        }

        let data = reqwest::get(url)
            .await
            .map_err(|_| format!("could not get image associated with attachment number {i}"))?;

        backend
            .file_write_all(
                format!("img{}", i).as_ref(),
                &data.bytes().await.map_err(|_| {
                    format!("could not interpret bytes of image of attachment number {i}")
                })?,
            )
            .unwrap();
        full_code.push_str(&format!("popunimg&frab\"img{i}\"\n"));
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
    match Primitive::from_format_name(f)
        .or_else(|| Primitive::from_glyph(f.chars().next().unwrap_or_default()))
        .or_else(|| Primitive::from_name(f))
    {
        Some(docs) => {
            let short = join_all(docs.doc().short.iter().map(|frag| async {
                format!(
                    "## {}",
                    print_doc_frag(frag, ctx.clone(), msg.clone()).await
                )
            }))
            .await
            .join("\n");

            let long = join_all(
                docs.doc()
                    .lines
                    .iter()
                    .take(5)
                    .map(|docs| async { print_docs(docs, ctx.clone(), msg.clone()).await }),
            )
            .await
            .join("\n");
            format!("\n{short}\n\n\n{long}\n\n([More information](https://uiua.org/docs/{}))", docs.name())
        }
        None => format!("No docs found for '{f}', did you spell it right? (For full docs, see [full docs](https://www.uiua.org/docs))"),
    }
}

async fn print_doc_frag(frag: &PrimDocFragment, ctx: Context, msg: Message) -> String {
    match frag {
        PrimDocFragment::Text(t) => t.clone(),
        PrimDocFragment::Code(t) => format!("`{t}`"),
        PrimDocFragment::Emphasis(t) => format!("_{t}_"),
        PrimDocFragment::Strong(t) => format!("**{t}**"),
        PrimDocFragment::Primitive { prim, named: _ } => {
            print_emoji(prim, ctx, msg).await
            //if *named {
            //    format!("{} `{}`", print_emoji(prim, ctx, msg), prim.name())
            //} else {
            //    print_emoji(prim)
            //}
        }
        PrimDocFragment::Link { text, url } => format!("[{text}]({url})"),
    }
}

async fn print_docs(line: &PrimDocLine, ctx: Context, msg: Message) -> String {
    match line {
        PrimDocLine::Text(vs) => join_all(
            vs.iter()
                .map(|frag| async { print_doc_frag(frag, ctx.clone(), msg.clone()).await }),
        )
        .await
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

pub async fn emoji_from_name(
    name: &str,
    ctx: Context,
    msg: Message,
) -> Result<Emoji, EmojiParseError> {
    dbg!(name);
    dbg!(Emoji::convert(ctx.clone(), msg.guild_id, Some(msg.channel_id), name).await)
}

async fn print_emoji(c: &Primitive, ctx: Context, msg: Message) -> String {
    if c.is_experimental() {
        c.names()
            .glyph
            .map(|g| g.to_string())
            .unwrap_or(c.name().to_string())
    } else {
        let name = c.name().split(' ').collect::<String>();
        let emoji = emoji_from_name(&name, ctx, msg).await;
        match emoji {
            Ok(e) => {
                trace!(name, "Succesfully got emoji");
                format!("<:{}:{}>", name, e.id)
            }
            Err(e) => {
                trace!(error = ?e, "Error getting emoji");
                name // it's a function with no glyph, like `&p`
            }
        }
    }
}

pub fn format_and_get_pad_link(code: &str) -> String {
    let encoded = URL_SAFE.encode(code);
    let link = format!("https://www.uiua.org/pad?src={}__{encoded}", uiua::VERSION);

    let result = format!("[pad]({link}) for: {}", highlight_code(&code));

    if result.len() <= MAX_MSG_LEN {
        trace!("Sending pad message normally");
        result
    } else {
        trace!("Pad message was too long, skipping source");
        format!("[pad]({link})")
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
    let result = run_uiua(strip_triple_ticks(code.trim()), &msg.attachments);

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
