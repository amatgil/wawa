use serenity::{
    all::{Context, Message},
    futures::{stream, StreamExt},
};
use std::fmt::Write;
use tracing::trace;
use uiua::{
    ast::Subscript,
    format::{format_str, FormatConfig},
    lsp::BindingDocsKind,
    PrimClass, Primitive, SpanKind,
};

use crate::emoji_from_name;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default)]
struct AnsiState {
    color: AnsiColor,
    bold: bool,
    //italic: bool,
    //dim: bool,
    //underline: bool,
    //blink: bool,
    //reverse: bool,
    //hide: bool,
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

impl AnsiState {
    fn style(&self, text: &str) -> String {
        if self.bold {
            format!("\x1B[{};1m{text}\x1B[0m", u8::from(self.color))
        } else {
            format!("\x1B[{}m{text}", u8::from(self.color))
        }
    }

    fn bold(self) -> Self {
        let mut new = self;
        new.bold = true;
        new
    }
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

    let output: String = uiua::lsp::Spans::from_input(&code)
        .spans
        .into_iter()
        .map(|s| {
            let text =
                &code[s.span.start.byte_pos as usize..s.span.end.byte_pos as usize].trim_end();

            let whitespace = (code[0..s.span.start.byte_pos as usize])
                .chars()
                .rev()
                .take_while(|c| c.is_whitespace())
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();

            let style: AnsiState = Span::from(s.value).into();

            if text.is_empty() {
                "".to_string()
            } else {
                format!("{}{}", whitespace, style.style(text))
            }
        })
        .collect();

    if output.is_empty() {
        trace!(?code, "Result of highlighting was empty");
        "<Empty code>".into()
    } else {
        trace!(?code, "Highlighted code successfully");
        format!("```ansi\n{}\n```", output)
    }
}

#[derive(Default)]
enum Span {
    Comment,
    String,
    Number,
    Label,
    Strand,
    Module,

    Constant,
    NoadicFun,
    StackFun,
    MonadicFun,
    DyadicFun,

    MonadicMod,
    DyadicMod,

    #[default]
    None,
}

impl From<SpanKind> for Span {
    fn from(spankind: SpanKind) -> Self {
        use SpanKind as SK;
        match spankind {
            SK::Primitive(prim, sub) => Span::from_prim_sub(Some(prim), sub),
            SK::String => Span::String,
            SK::Number => Span::Number,
            SK::Comment | SK::OutputComment => Span::Comment,
            SK::Strand => Span::Strand,
            SK::Label => Span::Label,
            SK::Subscript(prim, Some(sub)) => Span::from_prim_sub(prim, Some(sub)),
            SK::Obverse(..) => Span::MonadicMod,
            SK::Ident {
                docs: Some(docs), ..
            } => match docs.kind {
                BindingDocsKind::Constant(_) => Span::Constant,
                BindingDocsKind::Function { sig, .. } => match sig.args() {
                    0 => Span::NoadicFun,
                    1 => Span::MonadicFun,
                    2 => Span::DyadicFun,
                    _ => Span::default(),
                },
                BindingDocsKind::Modifier(args) => match args {
                    1 => Span::MonadicMod,
                    2 => Span::DyadicMod,
                    _ => Span::default(),
                },
                BindingDocsKind::Module { .. } => Span::Module,
                BindingDocsKind::Error => Span::default(),
            },
            _ => Span::default(),
        }
    }
}

impl From<Span> for AnsiState {
    fn from(span: Span) -> Self {
        match span {
            Span::Comment => AnsiColor::Gray.into(),
            Span::String => AnsiColor::Cyan.into(),
            Span::Number => AnsiState::from(AnsiColor::Red).bold(),
            Span::Label => AnsiState::from(AnsiColor::White).bold(),
            Span::Strand => AnsiColor::Gray.into(),
            Span::Module => AnsiState::from(AnsiColor::White).bold(),

            Span::Constant => AnsiColor::White.into(),
            Span::NoadicFun => AnsiColor::Red.into(),
            Span::StackFun => AnsiColor::White.into(),
            Span::MonadicFun => AnsiColor::Green.into(),
            Span::DyadicFun => AnsiColor::Blue.into(),
            Span::MonadicMod => AnsiColor::Yellow.into(),
            Span::DyadicMod => AnsiColor::Magenta.into(),

            Span::None => AnsiState::default(),
        }
    }
}

impl Span {
    fn from_prim(prim: Primitive, args: Option<usize>) -> Self {
        if let Some(args) = prim.modifier_args() {
            return if args == 1 {
                Self::MonadicMod
            } else {
                Self::DyadicMod
            };
        }

        if matches!(prim.class(), PrimClass::Stack | PrimClass::Debug)
            || prim == Primitive::Identity
        {
            return Self::StackFun;
        }

        args.or(prim.sig().map(|sig| sig.args()))
            .map(|args| match args {
                0 => Self::NoadicFun,
                1 => Self::MonadicFun,
                2 => Self::DyadicFun,
                _ => Self::None,
            })
            .unwrap_or_default()
    }

    fn from_prim_sub(prim: Option<Primitive>, sub: Option<Subscript>) -> Self {
        let args = prim
            .and_then(|prim| prim.subscript_sig(sub))
            .map(|sig| sig.args());
        prim.map(|prim| Self::from_prim(prim, args))
            .unwrap_or_default()
    }
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
