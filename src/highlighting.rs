use serenity::{
    all::{Context, Message},
    futures::{stream, StreamExt},
};
use std::fmt::Write;
use tracing::trace;
use uiua::{
    ast::Subscript,
    format::{format_str, FormatConfig},
    PrimClass, Primitive, SpanKind,
};

use crate::emoji_from_name;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default)]
struct AnsiState {
    color: AnsiColor,
    bold: bool,
    italic: bool,
    dim: bool,
    underline: bool,
    blink: bool,
    reverse: bool,
    hide: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default)]
enum AnsiColor {
    Gray, // Also black
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    #[default]
    White,
    Default,
    Reset,
}

impl From<AnsiColor> for u8 {
    fn from(color: AnsiColor) -> Self {
        match color {
            AnsiColor::Gray => 30,
            AnsiColor::Red => 31,
            AnsiColor::Green => 32,
            AnsiColor::Yellow => 33,
            AnsiColor::Blue => 34,
            AnsiColor::Magenta => 35,
            AnsiColor::Cyan => 36,
            AnsiColor::White => 37,
            AnsiColor::Default => 39,
            AnsiColor::Reset => 0,
        }
    }
}

impl From<AnsiColor> for AnsiState {
    fn from(color: AnsiColor) -> Self {
        AnsiState {
            color,
            ..Default::default()
        }
    }
}

use std::fmt;

impl fmt::Display for AnsiState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sugar = [
            (self.bold, 1),
            (self.dim, 2),
            (self.italic, 3),
            (self.underline, 4),
            (self.blink, 5),
            (self.reverse, 7),
            (self.hide, 8),
        ]
        .iter()
        .filter(|(b, _)| *b)
        .map(|(_, n)| n.to_string())
        .collect::<Vec<String>>()
        .join(";");

        if sugar.is_empty() {
            write!(f, "\x1B[{}m", u8::from(self.color))
        } else {
            write!(f, "\x1B[{};{}m", u8::from(self.color), sugar)
        }
    }
}

impl AnsiState {
    fn style(&self, s: &str) -> String {
        format!("{}{}\x1B[0m", self, s)
    }

    fn bold(&self) -> Self {
        let mut new = *self;
        new.bold = true;
        new
    }

    fn dim(&self) -> Self {
        let mut new = *self;
        new.dim = true;
        new
    }
}

fn prim_sub_style(prim: Option<Primitive>, sub: Option<Subscript>) -> AnsiState {
    let args = prim
        .and_then(|prim| prim.subscript_sig(sub))
        .map(|sig| sig.args() as i32);
    let style = prim
        .map(|prim| style_of_prim(prim, args))
        .unwrap_or_default();
    style
}

/// Returns code surrounded by ANSI backticks to fake highlighting
pub fn highlight_code(code: &str) -> String {
    let config = FormatConfig::default();
    let code = match format_str(code, &config) {
        Ok(s) => s.output,
        Err(e) => {
            tracing::error!(?e, "Error while formatting line for pad");
            return format!("```\n{e}\n```");
        }
    };

    let spans: Vec<_> = uiua::lsp::Spans::from_input(&code).spans;

    let r: String = spans
        .into_iter()
        .map(|s| {
            let text = &code[s.span.start.byte_pos as usize..s.span.end.byte_pos as usize];

            let whitespace = (&code[0..s.span.start.byte_pos as usize])
                .chars()
                .rev()
                .take_while(|c| c.is_whitespace())
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();

            use SpanKind as SK;
            let (code, style): (&str, _) = match s.value {
                SK::Primitive(prim, sub) => (&prim.to_string(), prim_sub_style(Some(prim), sub)),
                SK::String => (text, AnsiColor::Cyan.into()),
                SK::Number => (text, AnsiState::from(AnsiColor::Red).bold()),
                SK::Comment => (text, AnsiState::from(AnsiColor::Gray).dim()),
                SK::OutputComment => (text, AnsiState::from(AnsiColor::White).dim()),
                SK::Strand => (text, AnsiColor::White.into()),
                SK::Label => (
                    text,
                    AnsiState {
                        color: AnsiColor::White,
                        bold: true,
                        italic: true,
                        dim: true,
                        blink: true,
                        ..Default::default()
                    },
                ),
                SK::Subscript(prim, Some(sub)) => {
                    (&sub.to_string(), prim_sub_style(prim, Some(sub)))
                }
                SK::Obverse(..) => (text, AnsiColor::Yellow.into()),
                _ => (text, AnsiState::default()),
            };

            if code.chars().all(char::is_whitespace) {
                "".to_string()
            } else {
                format!("{}{}", whitespace, style.style(code.trim_end()))
            }
        })
        .collect();

    if r.is_empty() {
        trace!(?code, "Result of highlighting was empty");
        "<Empty code>".into()
    } else {
        trace!(?code, "Highlighted code successfully");
        format!("```ansi\n{}\n```", r)
    }
}

fn style_of_prim(prim: Primitive, sig: Option<i32>) -> AnsiState {
    let noadic = AnsiColor::Red.into();
    let monadic = AnsiColor::Green.into();
    let monadic_mod = AnsiColor::Yellow.into();
    let dyadic_mod = AnsiColor::Magenta.into();
    let dyadic = AnsiColor::Blue.into();
    let constant = AnsiState::from(AnsiColor::Red).bold();

    if prim == Primitive::Identity {
        return AnsiState::default();
    }

    match prim.class() {
        PrimClass::Stack | PrimClass::Debug if prim.modifier_args().is_none() => None,
        PrimClass::Constant => Some(constant),
        _ => {
            if let Some(margs) = prim.modifier_args() {
                Some(if margs == 1 { monadic_mod } else { dyadic_mod })
            } else {
                match sig.or(prim.args().map(|a| a as i32)) {
                    Some(0) => Some(noadic),
                    Some(1) => Some(monadic),
                    Some(2) => Some(dyadic),
                    _ => None,
                }
            }
        }
    }
    .unwrap_or_default()
}

pub async fn emojificate(code: &str, msg: Message, ctx: Context) -> String {
    let spans: Vec<_> = uiua::lsp::Spans::from_input(code).spans;

    let mut r: String = stream::iter(spans.into_iter())
        .fold((String::new(), 0), |(mut out, mut last_cursor), s| {
            let ctxclone = ctx.clone();
            let msgclone = msg.clone();
            {
                async move {
                    let newlines_skipped = code
                        .bytes()
                        .skip(last_cursor as usize)
                        .take(s.span.start.byte_pos as usize - last_cursor as usize)
                        .filter(|c| *c == b'\n')
                        .count();
                    let text = &code[s.span.start.byte_pos as usize..s.span.end.byte_pos as usize];
                    last_cursor = s.span.end.byte_pos;

                    let fmtd = match s.value {
                        SpanKind::Primitive(p, _) => emoji_from_name(p.name(), ctxclone, msgclone)
                            .await
                            .map(|e| format!("<:{}:{}>", e.name, e.id))
                            .unwrap_or_else(|_| format!("`{}`", p.name())),
                        SpanKind::String => format!("`{text}`"),
                        SpanKind::Number => format!("`{text}`"),
                        SpanKind::Comment => format!("`{text}`"),
                        SpanKind::OutputComment => format!("`{text}`"),
                        SpanKind::Strand => format!("`{text}`"),
                        SpanKind::Ident { .. } => {
                            if text == "Lena" {
                                emoji_from_name("lena", ctxclone, msgclone)
                                    .await
                                    .map(|e| format!("<:{}:{}>", e.name, e.id))
                                    .unwrap_or_else(|_| "<lena emoji should go here>".to_string())
                            } else {
                                format!("`{text}`")
                            }
                        }
                        SpanKind::Label => format!("`{text}`"),
                        SpanKind::Signature => format!("`{text}`"),
                        SpanKind::Whitespace => format!("{text}"), // no backticks for space
                        SpanKind::Placeholder(..) => format!("`{text}`"),
                        SpanKind::Delimiter => format!("`{text}`"),
                        SpanKind::FuncDelim(..) => format!("`{text}`"),
                        SpanKind::ImportSrc(..) => format!("`{text}`"),
                        SpanKind::Subscript(_, Some(x)) => {
                            let subs_text: String = (x.to_string().chars())
                                .map(|c| uiua::SUBSCRIPT_DIGITS[(c as u32 as u8 - b'0') as usize])
                                .collect();
                            format!("`{subs_text}`")
                        }
                        SpanKind::Subscript(_, None) => format!("`{text}`"),
                        SpanKind::Obverse(..) => format!("`{text}`"),
                        SpanKind::MacroDelim(..) => format!("`{text}`"),
                        SpanKind::LexOrder => format!("`{text}`"),
                    };
                    let _ = write!(out, "{}{} ", "\n".repeat(newlines_skipped), fmtd);
                    (out, last_cursor)
                }
            }
        })
        .await
        .0;

    if r.is_empty() {
        trace!(?code, "Result of highlighting was empty");
        r = "<Empty code>".into();
    } else {
        trace!(?code, "Highlighted code successfully");
    }
    r
}
