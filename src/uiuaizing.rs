use std::time::Duration;

use std::collections::HashMap;
use uiua::{PrimClass, PrimDocFragment, PrimDocLine, Primitive, Signature, Uiua};
use uiua::format::*;
use base64::{Engine};

use base64::engine::general_purpose::URL_SAFE;

const DEFAULT_EXECUTION_LIMIT: Duration = Duration::from_secs(2);

pub fn run_uiua(code: &str) -> String {
    if code.is_empty() {
        return "Cannot run empty code".into();
    }
    let mut runtime = Uiua::with_safe_sys().with_execution_limit(DEFAULT_EXECUTION_LIMIT).with_;

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
                        prim.to_string()
                    }
                }
                PrimDocFragment::Link { text, url } => format!("[{text}]({url})"),
            })
            .collect::<Vec<String>>()
            .join("\n"),
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

fn ansi_code(code: u8) -> String {
    format!("\x1B[38;5;{}m", code)
}
fn reset_ansi() -> String {
    "\x1b[0m".into()
}

/// Returns code surrounded by ANSI backticks to fake highlighting
pub fn highlight_code(code: &str) -> String {
    let cs: String = code.chars().map(|c| print_char(c)).collect();
    dbg!(&cs);
    println!("{cs}");
    format!("```ansi\n{cs}\n```")
}

fn print_char(c: char) -> String {
    let g = match Primitive::from_glyph(c) {
        Some(g) => g,
        None => return c.to_string(),
    };

    let codes = HashMap::from([
        ("Black", 30),
        ("Red", 31),
        ("Green", 32),
        ("Yellow", 33),
        ("Blue", 34),
        ("Magenta", 35),
        ("Cyan", 36),
        ("White", 37),
        ("Orange", 37),
        ("Reset", 0),
    ]);
    let noadic = "Red";
    let monadic = "Green";
    let monadic_mod = "Yellow";
    let dyadic_mod = "Magenta";
    let dyadic = "Blue";

    let for_prim = |prim: Primitive, sig: Option<Signature>| match prim.class() {
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
    };

    let col_s = for_prim(g, g.signature()).unwrap_or("White");
    format!(
        "\x1B[1;3;{}m{}\x1B[{}m",
        codes.get(col_s).unwrap(),
        g,
        codes.get("Reset").unwrap(),
    )
}

fn print_emoji(c: &Primitive) -> String {
    format!(":{}:", c.name())
}

pub fn format_and_get_pad_link(code: &str) -> String {
    let config = FormatConfig::default();
    let formatted = format_str(code, &config).unwrap().output;

    let encoded = URL_SAFE.encode(code);
    let link = format!("https://www.uiua.org/pad?src={}__{encoded}", uiua::VERSION);

    format!("[pad]({link}) for: {}", highlight_code(&formatted))
}
