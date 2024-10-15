use std::time::Duration;

use uiua::PrimDocLine;
use uiua::PrimDocFragment;
use uiua::Primitive;
use uiua::Uiua;

const DEFAULT_EXECUTION_LIMIT: Duration = Duration::from_secs(4);

pub fn run_uiua(code: &str) -> String {
    if code.is_empty() {
        return "Cannot run empty code".into();
    }
    let mut runtime = Uiua::with_safe_sys().with_execution_limit(DEFAULT_EXECUTION_LIMIT);
    match runtime.run_str(code) {
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
        PrimDocLine::Text(vs) => {
            vs.into_iter().map(|v| match v {
                PrimDocFragment::Text(t) => t.clone(),
                PrimDocFragment::Code(t) => format!("```\n{t}\n```"),
                PrimDocFragment::Emphasis(t) => format!("_{t}_"),
                PrimDocFragment::Strong(t) => format!("**{t}**"),
                PrimDocFragment::Primitive { prim, named } => {
                    if *named {
                        prim.format().to_string()
                    } else {
                        prim.to_string()
                    }
                },
                PrimDocFragment::Link { text, url } => format!("[{text}]({url})"),
            }).collect::<Vec<String>>().join("\n")
        }
        PrimDocLine::Example(e) => {
            format!(
                "```
{}  returns  {}
```
",
                e.input(),
                match e.output().as_ref().map(|vs| vs.join(";")) {
                    Ok(l) => l,
                    Err(l) => l.clone()
                }
            )
        }
    }
}



/// Returns code surrounded by ANSI backticks to fake highlighting
fn highlight_code(code: &str) -> String {
    //https://github.com/uiua-lang/uiua/blob/main/src/main.rs#L1045
    todo!()
}
