use std::time::Duration;

use base64::Engine;
use uiua::format::*;
use uiua::{PrimClass, PrimDocFragment, PrimDocLine, Primitive, Signature, SpanKind, Uiua};
use crate::*;

use base64::engine::general_purpose::URL_SAFE;

const DEFAULT_EXECUTION_LIMIT: Duration = Duration::from_secs(2);

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


