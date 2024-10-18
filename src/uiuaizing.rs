use std::time::Duration;

use crate::*;
use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use std::fmt::Write;
use tracing::trace;
use uiua::format::*;
use uiua::{PrimDocFragment, PrimDocLine, Primitive, Uiua};

const MIN_AUTO_IMAGE_DIM: usize = 30;
const MAX_STACK_VALS_DISPLAYED: usize = 10;
const DEFAULT_EXECUTION_LIMIT: Duration = Duration::from_secs(2);

pub enum OutputItem {
    /// Audio, containing encoded OGG Vorbis bytes.
    Audio(Box<[u8]>),
    /// Static image data, containing encoded PNG bytes.
    Image(Box<[u8]>),
    /// Miscellaneous value.
    Misc(uiua::Value),
    /// "Hey, there's {n} more values!" indicator
    Continuation(u32),
    // TODO: images, gifs, you know the drill
}

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
            Ok(OutputItem::Audio(sink.into_boxed_slice()))
        }
        use uiua::encode::*;
        use uiua::Value;

        if value.shape().last().is_some_and(|&n| n >= 44100 / 4)
            && matches!(&value, Value::Num(arr) if arr.elements().all(|x| x.abs() <= 5.0))
        {
            if let Ok(this) = try_from_ogg(&value) {
                return this;
            }
        }
        if let Ok(image) = value_to_image(&value) {
            if image.width() >= MIN_AUTO_IMAGE_DIM as u32
                && image.height() >= MIN_AUTO_IMAGE_DIM as u32
            {
                if let Ok(bytes) = image_to_bytes(&image, image::ImageOutputFormat::Png) {
                    return OutputItem::Image(bytes.into());
                }
            }
        }

        Self::Misc(value)
    }
}

pub fn run_uiua(code: &str) -> Result<Vec<OutputItem>, String> {
    trace!(code, "Starting to execute uiua code");
    if code.is_empty() {
        return Err("Cannot run empty code".into());
    }

    let mut runtime = Uiua::with_safe_sys().with_execution_limit(DEFAULT_EXECUTION_LIMIT);
    let exp_code = &format!("# Experimental!\n{code}");

    match runtime.run_str(exp_code) {
        Ok(_c) => {
            trace!(code, "Code ran successfully");
            let mut stack = runtime.take_stack();
            let stack_len = stack.len();
            if stack_len > MAX_STACK_VALS_DISPLAYED {
                stack.truncate(MAX_STACK_VALS_DISPLAYED);
            }
            Ok(stack
                .into_iter()
                .map(|val| val.into())
                .chain((stack_len > MAX_STACK_VALS_DISPLAYED).then(|| {
                    OutputItem::Continuation((stack_len - MAX_STACK_VALS_DISPLAYED) as u32)
                }))
                .collect())
        }
        Err(e) => {
            trace!(code, "Code ran Unsuccessfully");
            Err(format!("Error while running: {e} "))
        }
    }
}

pub fn get_docs(f: &str) -> String {
    match Primitive::from_format_name(f)
        .or_else(|| Primitive::from_glyph(f.chars().next().unwrap_or_default()))
        .or_else(|| Primitive::from_name(f))
    {
        Some(docs) => {
            let short = docs
                .doc()
                .short
                .iter()
                .map(print_doc_frag)
                .map(|s| format!("## {s}"))
                .collect::<Vec<String>>()
                .join("\n");

            let long = docs
                .doc()
                .lines
                .iter()
                .take(5)
                .map(print_docs)
                .collect::<Vec<String>>()
                .join("\n");
            format!("\n{short}\n\n\n{long}\n\n([More information](https://uiua.org/docs/{f}))")
        }
        None => format!("No docs found for '{f}', did you spell it right?"),
    }
}

fn print_doc_frag(frag: &PrimDocFragment) -> String {
    match frag {
        PrimDocFragment::Text(t) => t.clone(),
        PrimDocFragment::Code(t) => format!("`{t}`"),
        PrimDocFragment::Emphasis(t) => format!("_{t}_"),
        PrimDocFragment::Strong(t) => format!("**{t}**"),
        PrimDocFragment::Primitive { prim, named } => {
            if *named {
                format!("{} `{}`", print_emoji(prim), prim.name())
            } else {
                print_emoji(prim)
            }
        }
        PrimDocFragment::Link { text, url } => format!("[{text}]({url})"),
    }
}

fn print_docs(line: &PrimDocLine) -> String {
    match line {
        PrimDocLine::Text(vs) => vs
            .iter()
            .map(print_doc_frag)
            .collect::<Vec<String>>()
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
                    let _ = writeln!(out, "# {l}\n");
                    out
                })
            );

            highlight_code(&text).to_string()
        }
    }
}

fn print_emoji(c: &Primitive) -> String {
    if c.is_experimental() {
        c.names()
            .glyph
            .map(|g| g.to_string())
            .unwrap_or(c.name().to_string())
    } else {
        let spaceless_name = c.name().split(' ').collect::<String>();
        format!(":{}:", spaceless_name)
    }
}

pub fn format_and_get_pad_link(code: &str) -> String {
    let config = FormatConfig::default();
    let formatted = format_str(code, &config).unwrap().output;

    let encoded = URL_SAFE.encode(code);
    let link = format!("https://www.uiua.org/pad?src={}__{encoded}", uiua::VERSION);

    format!("[pad]({link}) for: {}", highlight_code(&formatted))
}
