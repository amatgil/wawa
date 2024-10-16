use std::default;
use std::time::Duration;

use base64::Engine;
use uiua::format::*;
use uiua::{PrimClass, PrimDocFragment, PrimDocLine, Primitive, Signature, SpanKind, Uiua};

use base64::engine::general_purpose::URL_SAFE;

const DEFAULT_EXECUTION_LIMIT: Duration = Duration::from_secs(2);

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

#[derive(Debug, Clone, Copy, Default)]
enum AnsiColor {
    Black,
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
        format!("\x1B[{};{}m", self.color.ansi_code(), sugar)
    }
}

impl AnsiColor {
    fn ansi_code(&self) -> u8 {
        match self {
            Self::Black => 30,
            Self::Red => 31,
            Self::Green => 32,
            Self::Yellow => 33,
            Self::Blue => 34,
            Self::Magenta => 35,
            Self::Cyan => 26,
            Self::White => 37,
            Self::Default => 39,
            Self::Reset => 0,
        }
    }
}

pub fn run_uiua(code: &str) -> String {
    if code.is_empty() {
        return "Cannot run empty code".into();
    }
    let mut runtime = Uiua::with_safe_sys().with_execution_limit(DEFAULT_EXECUTION_LIMIT);

    let exp_code = &format!("# Experimental!\n{code}");

    match runtime.run_str(exp_code) {
        Ok(_c) => {
            let r = runtime
                .take_stack()
                .into_iter()
                .take(10)
                .map(|v| v.show())
                .collect::<Vec<String>>()
                .join("\n");
            format!("```\n{r}\n```")
        }
        Err(e) => format!(
            "Error while running:
```
{e}
```
"
        ),
    }
}

pub fn get_docs(f: &str) -> String {
    match Primitive::from_format_name(f) {
        Some(docs) => {
            let d = docs
                .doc()
                .lines
                .iter()
                .take(4)
                .map(print_docs)
                .collect::<Vec<String>>()
                .join("\n");
            format!("Documentation ([link](https://uiua.org/docs/{f})):\n{d}")
        }
        None => format!("No docs found for '{f}', did you spell it right?"),
    }
}

fn print_docs(line: &PrimDocLine) -> String {
    match line {
        PrimDocLine::Text(vs) => vs
            .into_iter()
            .map(|v| match v {
                PrimDocFragment::Text(t) => t.clone(),
                PrimDocFragment::Code(t) => format!("```\n{t}\n```"),
                PrimDocFragment::Emphasis(t) => format!("_{t}_"),
                PrimDocFragment::Strong(t) => format!("**{t}**"),
                PrimDocFragment::Primitive { prim, named } => {
                    if *named {
                        format!("{} {}", print_emoji(prim), prim.name())
                    } else {
                        print_emoji(prim)
                        //prim.to_string()
                    }
                }
                PrimDocFragment::Link { text, url } => format!("[{text}]({url})"),
            })
            .collect::<Vec<String>>()
            .join(" "),
        PrimDocLine::Example(e) => {
            format!(
                "```
{}
# {}
```
",
                e.input(),
                match e.output().as_ref().map(|vs| vs.join(";")) {
                    Ok(l) => l,
                    Err(l) => l.clone(),
                }
            )
        }
    }
}

//fn ansi_code(code: u8) -> String {
//    format!("\x1B[38;5;{}m", code)
//}
//fn reset_ansi() -> String {
//    "\x1b[0m".into()
//}

/// Returns code surrounded by ANSI backticks to fake highlighting
pub fn highlight_code(code: &str) -> String {
    let spans: Vec<_> = uiua::lsp::spans(code).0;
    let r: String = spans
        .into_iter()
        .map(|s| {
            let text = &code[s.span.start.byte_pos as usize..s.span.end.byte_pos as usize];
            match s.value {
                SpanKind::Primitive(p, sig) => print_prim(p, sig),
                SpanKind::String => with_style(text, AnsiState::just_color(AnsiColor::Blue)),
                SpanKind::Number => with_style(text, AnsiState::just_color(AnsiColor::White)),
                SpanKind::Comment => with_style(
                    text,
                    AnsiState {
                        color: AnsiColor::White,
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
                SpanKind::Ident { docs, original } => {
                    "[wawa doesn't know what this [ident] is, please report]".to_string()
                }
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
                SpanKind::Signature => with_style(text, AnsiState::just_color(AnsiColor::White)),
                SpanKind::Whitespace => with_style(text, AnsiState::just_color(AnsiColor::White)),
                SpanKind::Placeholder(p) => {
                    format!("[wawa doesn't know what {p} is, please report]")
                }
                SpanKind::Delimiter => todo!(),
                SpanKind::FuncDelim(sig, set_inv) => todo!(),
                SpanKind::ImportSrc(src) => todo!(),
                SpanKind::Subscript(prim, n) => todo!(),
                SpanKind::Obverse(set_inv) => todo!(),
            }
        })
        .collect();

    dbg!(&code);
    let r = dbg!(format!("```ansi\n{}\n```", r));
    println!("{r}");
    r
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

pub fn with_style(s: &str, ansi: AnsiState) -> String {
    format!("{}{}\x1B[0m", ansi.ansi_str(), s)
}

fn print_prim(prim: Primitive, sig: Option<Signature>) -> String {
    let noadic = AnsiColor::Red;
    let monadic = AnsiColor::Green;
    let monadic_mod = AnsiColor::Yellow;
    let dyadic_mod = AnsiColor::Magenta;
    let dyadic = AnsiColor::Blue;

    let col = match prim.class() {
        PrimClass::Stack | PrimClass::Debug if prim.modifier_args().is_none() => None,
        PrimClass::Constant => None,
        _ => {
            if let Some(margs) = prim.modifier_args() {
                Some(if margs == 1 { monadic_mod } else { dyadic_mod })
            } else {
                match sig.map(|sig| sig.args).or(prim.args()) {
                    Some(0) => Some(noadic),
                    Some(1) => Some(monadic),
                    Some(2) => Some(dyadic),
                    _ => None,
                }
            }
        }
    }
    .unwrap_or(AnsiColor::White);

    with_style(&prim.to_string(), AnsiState::just_color(col))
}
