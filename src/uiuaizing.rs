use std::time::Duration;

use crate::*;
use base64::Engine;
use uiua::format::*;
use uiua::{PrimDocFragment, PrimDocLine, Primitive, Uiua};

use base64::engine::general_purpose::URL_SAFE;
use std::sync::LazyLock;
use std::collections::HashMap;

const DEFAULT_EXECUTION_LIMIT: Duration = Duration::from_secs(2);
const EMOJI_IDS: &'static str = include_str!("../assets/glyphlist.txt");
static EMOJI_MAP: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| {
    EMOJI_IDS.lines().map(|l| {
        let space_idx = l.bytes().position(|c| c == b' ').expect("EMOJI_IDS are malformed");
        let (a, b) = l.split_at(space_idx);
        (a, &b[1..])
    }).collect::<HashMap<&str, &str>>()
});

pub fn run_uiua(code: &str) -> String {
    if code.is_empty() {
        return "Cannot run empty code".into();
    }

    let mut runtime = Uiua::with_safe_sys().with_execution_limit(DEFAULT_EXECUTION_LIMIT);
    let exp_code = &format!("# Experimental!\n{code}");
    let r = match runtime.run_str(exp_code) {
        Ok(_c) => runtime
            .take_stack()
            .into_iter()
            .take(10)
            .map(|v| v.show())
            .collect::<Vec<String>>()
            .join("\n"),
        Err(e) => format!("Error while running: {e} "),
    };

    if r.contains("```") {
        "Output contained triple backticks, which I disallow".to_string()
    } else {
        if r == "" {
            "<Empty stack>".to_string()
        } else {
            format!("```\n{r}\n```")
        }
    }
}

pub fn get_docs(f: &str) -> String {
    match Primitive::from_format_name(f)
        .or_else(|| Primitive::from_glyph(f.chars().next().unwrap_or_default()))
        .or_else(|| Primitive::from_name(f))
    {
        Some(docs) => {
            let d = docs
                .doc()
                .lines
                .iter()
                .take(5)
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
                    Ok(l) => format!("> {l}"),
                    Err(l) => format!("> {l}"),
                }
            )
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
        if let Some(id) = EMOJI_MAP.get(&*spaceless_name) {
            format!("<:{}:{}>", spaceless_name, id)
        } else {
            format!("<{}>", spaceless_name)
        }
    }
}

pub fn format_and_get_pad_link(code: &str) -> String {
    let config = FormatConfig::default();
    let formatted = format_str(code, &config).unwrap().output;

    let encoded = URL_SAFE.encode(code);
    let link = format!("https://www.uiua.org/pad?src={}__{encoded}", uiua::VERSION);

    format!("[pad]({link}) for: {}", highlight_code(&formatted))
}
