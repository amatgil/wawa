use std::time::Duration;

use uiua::PrimClass;
use uiua::PrimDocLine;
use uiua::PrimDocFragment;
use uiua::Primitive;
use uiua::Signature;
use uiua::Uiua;
use std::collections::HashMap;

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
                        format!("{} {}", print_emoji(prim), prim.name())
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
{}
# {}
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


//fn ansi_code([r, g, b]: [f32; 3]) -> String {
//    format!("\x1B[38;2;{{{}}};{{{}}};{{{}}}m",
//            (255.0 * r) as u8,
//            (255.0 * g),
//            (255.0 * b))
//}
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


// TODO: highlighting can go here, even
fn print_char(c: char) -> String {
    let g = match Primitive::from_glyph(c) {
        Some(g) => g,
        None => return c.to_string()
    };


    //let cols: HashMap<&str, [f32; 3]> = HashMap::from([
    //    ("White", [1.0, 1.0, 1.0]),
    //    ("Black", [0.0, 0.0, 0.0]),
    //    ("Red", [1.0, 0.0, 0.0]),
    //    ("Orange", [1.0, 0.5, 0.0]),
    //    ("Yellow", [1.0, 1.0, 0.0]),
    //    ("Green", [0.0, 1.0, 0.0]),
    //    ("Cyan", [0.0, 1.0, 1.0]),
    //    ("Blue", [0.0, 0.0, 1.0]),
    //    ("Purple", [0.5, 0.0, 1.0]),
    //    ("Magenta", [1.0, 0.0, 1.0]),
    //]);
    let codes = HashMap::from([
        ("Black" , 30),
        ("Red" , 31),
        ("Green" , 32),
        ("Yellow" , 33),
        ("Blue" , 34),
        ("Magenta" , 35),
        ("Cyan" , 36),
        ("White" , 37),
        ("Reset" , 0)]);
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
    format!("\x1B[{}m{}\x1B[{}m",
            codes.get(col_s).unwrap(),
            g,
            codes.get("Reset").unwrap(),
    )
}

fn print_emoji(c: &Primitive) -> String {
    format!(":{}:", c.name())
}
