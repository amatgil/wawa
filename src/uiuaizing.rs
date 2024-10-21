use std::time::Duration;

use crate::backend::{NativisedWebBackend, OutputItem};
use crate::*;
use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use serenity::all::{ArgumentConvert, Context, Emoji, EmojiParseError, Message};
use serenity::futures::future::join_all;
use std::fmt::Write;
use std::str;
use tracing::{error, trace};
use uiua::format::*;
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
        use uiua::encode::*;
        use uiua::Value;

        // Audio?
        if value.shape().last().is_some_and(|&n| n >= 44100 / 4)
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
                if let Ok(bytes) = image_to_bytes(&image, image::ImageOutputFormat::Png) {
                    trace!("Turning image into bytes");
                    return OutputItem::Image(bytes, None);
                }
            }
        }
        // Gif
        if let Ok(gif) = value_to_gif_bytes(&value, 16.0) {
            match value.shape().dims() {
                &[f, h, w] | &[f, h, w, _]
                    if h >= MIN_AUTO_IMAGE_DIM && w >= MIN_AUTO_IMAGE_DIM && f >= 5 =>
                {
                    trace!("Turning gif into bytes");
                    return OutputItem::Gif(gif, None);
                }
                _ => {}
            }
        }

        OutputItem::String(value.to_string())
    }
}

/// Returns (stdout, top-most elements of stack)
pub fn run_uiua(code: &str) -> Result<(Vec<OutputItem>, Vec<OutputItem>), String> {
    trace!(code, "Starting to execute uiua code");
    if code.is_empty() {
        return Err("Cannot run empty code".into());
    }

    let mut runtime = Uiua::with_backend(NativisedWebBackend::default())
        .with_execution_limit(DEFAULT_EXECUTION_LIMIT);
    let exp_code = &format!("# Experimental!\n{code}");

    match runtime.run_str(exp_code) {
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
            format!("\n{short}\n\n\n{long}\n\n([More information](https://uiua.org/docs/{f}))")
        }
        None => format!("No docs found for '{f}', did you spell it right?"),
    }
}

async fn print_doc_frag(frag: &PrimDocFragment, ctx: Context, msg: Message) -> String {
    match frag {
        PrimDocFragment::Text(t) => t.clone(),
        PrimDocFragment::Code(t) => format!("`{t}`"),
        PrimDocFragment::Emphasis(t) => format!("_{t}_"),
        PrimDocFragment::Strong(t) => format!("**{t}**"),
        PrimDocFragment::Primitive { prim, named } => {
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
    dbg!(Emoji::convert(ctx.clone(), msg.guild_id, Some(msg.channel_id), &name).await)
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
                trace!(name, "Succesfully got emoji")
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
    let config = FormatConfig::default();
    let formatted = match format_str(code, &config) {
        Ok(s) => s.output,
        Err(e) => {
            error!(?e, "Error while formatting line for pad");
            return format!("Internal uiua error while formatting source: `{e}`");
        }
    };

    let encoded = URL_SAFE.encode(code);
    let link = format!("https://www.uiua.org/pad?src={}__{encoded}", uiua::VERSION);

    let result = format!("[pad]({link}) for: {}", highlight_code(&formatted));

    if result.len() <= MAX_MSG_LEN {
        trace!("Sending pad message normally");
        result
    } else {
        trace!("Pad message was too long, skipping source");
        format!("[pad]({link})")
    }
}
