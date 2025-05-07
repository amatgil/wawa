use serenity::{
    all::{Context, Message},
    futures::{stream, StreamExt},
};
use std::fmt::Write;
use tracing::trace;
use uiua::{PrimClass, Primitive, SpanKind};

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

impl AnsiState {
    fn just_color(col: AnsiColor) -> Self {
        AnsiState {
            color: col,
            ..Default::default()
        }
    }
    fn ansi_str(&self) -> String {
        let sugar: String = [
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
            format!("\x1B[{}m", self.color.ansi_code())
        } else {
            format!("\x1B[{};{}m", self.color.ansi_code(), sugar)
        }
    }
}

impl AnsiColor {
    fn ansi_code(&self) -> u8 {
        match self {
            Self::Gray => 30,
            Self::Red => 31,
            Self::Green => 32,
            Self::Yellow => 33,
            Self::Blue => 34,
            Self::Magenta => 35,
            Self::Cyan => 36,
            Self::White => 37,
            Self::Default => 39,
            Self::Reset => 0,
        }
    }
}

fn with_style(s: &str, ansi: AnsiState) -> String {
    format!("{}{}\x1B[0m", ansi.ansi_str(), s)
}

/// Returns code surrounded by ANSI backticks to fake highlighting
pub fn highlight_code(code: &str) -> String {
    return "```Highlighter broke, down for maintenence (use wawa!show instead)```".to_string();

    let spans: Vec<_> = uiua::lsp::Spans::from_input(code).spans;
    let mut last_cursor: u32 = 0;
    let mut r: String = spans.into_iter().fold(String::new(), |mut out, s| {
        let newlines_skipped = code
            .bytes()
            .skip(last_cursor as usize)
            .take(s.span.start.byte_pos as usize - last_cursor as usize)
            .filter(|c| *c == b'\n')
            .count();
        let text = &code[s.span.start.byte_pos as usize..s.span.end.byte_pos as usize];
        last_cursor = s.span.end.byte_pos;

        let fmtd = match s.value {
            SpanKind::Primitive(p, sig) => print_prim(p, sig.map(|s| s.n()).flatten()),
            SpanKind::String => with_style(text, AnsiState::just_color(AnsiColor::Cyan)),
            SpanKind::Number => with_style(
                text,
                AnsiState {
                    color: AnsiColor::Red,
                    bold: true,
                    ..Default::default()
                },
            ),
            SpanKind::Comment => with_style(
                text,
                AnsiState {
                    color: AnsiColor::Gray,
                    dim: true,
                    ..Default::default()
                },
            ),
            SpanKind::OutputComment => with_style(
                text,
                AnsiState {
                    color: AnsiColor::White,
                    dim: true,
                    ..Default::default()
                },
            ),
            SpanKind::Strand => with_style(text, AnsiState::just_color(AnsiColor::White)),
            SpanKind::Ident { .. } => with_style(text, AnsiState::default()),
            SpanKind::Label => with_style(
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
            SpanKind::Signature => with_style(text, AnsiState::default()),
            SpanKind::Whitespace => with_style(text, AnsiState::default()),
            SpanKind::Placeholder(..) => with_style(text, AnsiState::default()),
            SpanKind::Delimiter => with_style(text, AnsiState::default()),
            SpanKind::FuncDelim(..) => with_style(text, AnsiState::default()),
            SpanKind::ImportSrc(..) => with_style(text, AnsiState::default()),
            SpanKind::Subscript(prim, Some(x)) => {
                // TODO: this does not highlight the attached primitive correctly
                let subs_text: String = (x.to_string().chars())
                    .map(|c| {
                        if c == '-' {
                            '₋'
                        } else {
                            uiua::SUBSCRIPT_DIGITS[(c as u32 as u8 - b'0') as usize]
                        }
                    })
                    .collect();
                let style = prim
                    .map(|p| style_of_prim(p, p.sig().map(|s| s.args() as i32)))
                    .unwrap_or_default();
                with_style(&subs_text, style)
            }
            SpanKind::Subscript(_, None) => with_style(text, AnsiState::default()),
            SpanKind::Obverse(..) => with_style(text, AnsiState::default()),
            SpanKind::MacroDelim(..) => with_style(text, AnsiState::default()),
            SpanKind::LexOrder => with_style(text, AnsiState::default()),
        };
        let _ = write!(out, "{}{}", "\n".repeat(newlines_skipped), fmtd);
        out
    });

    if r.is_empty() {
        trace!(?code, "Result of highlighting was empty");
        r = "<Empty code>".into();
    } else {
        trace!(?code, "Highlighted code successfully");
        r = format!("```ansi\n{}\n```", r);
    }
    r
}

fn style_of_prim(prim: Primitive, sig: Option<i32>) -> AnsiState {
    let noadic = AnsiState::just_color(AnsiColor::Red);
    let monadic = AnsiState {
        color: AnsiColor::Green,
        ..Default::default()
    };
    let monadic_mod = AnsiState {
        color: AnsiColor::Yellow,
        ..Default::default()
    };
    let dyadic_mod = AnsiState {
        color: AnsiColor::Magenta,
        ..Default::default()
    };
    let dyadic = AnsiState {
        color: AnsiColor::Blue,
        ..Default::default()
    };
    let constant = AnsiState {
        color: AnsiColor::Red,
        bold: true,
        ..Default::default()
    };

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
    .unwrap_or(AnsiState::default())
}

fn print_prim(prim: Primitive, sig: Option<i32>) -> String {
    let style = style_of_prim(prim, sig);

    with_style(&prim.to_string(), style)
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
