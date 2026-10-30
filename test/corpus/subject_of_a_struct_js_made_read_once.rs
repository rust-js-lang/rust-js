// A `match`'s or an `if let`'s subject, a field of a struct JS made, which
// a getter may answer, is read once, as Rust reads it, where it's tested:
// `const margin = style.margin; if (margin != null)`, its bindings read of
// that, not the getter again (#430; ADR 1022, amended). Natively, the
// binding's body makes what JS's `JSON.parse` gives.

struct Style {
    margin: Option<&'static str>,
    inner: Option<Option<&'static str>>,
}

#[cfg_attr(rust_js, rust_js::link_name = "JSON.parse")]
fn parse(text: &str) -> &'static Style {
    let field = |name: &str| {
        let key = format!("\"{name}\":\"");
        let at = text.find(&key)? + key.len();
        let end = text[at..].find('"')?;
        Some(&*Box::leak(text[at..at + end].to_string().into_boxed_str()))
    };
    let inner = field("inner").map(Some);
    Box::leak(Box::new(Style {
        margin: field("margin"),
        inner,
    }))
}

fn describe(style: &Style, out: &mut Vec<String>) {
    if let Some(margin) = style.margin {
        out.push(format!("margin {margin}"));
    }
    match style.margin {
        Some(margin) if margin.len() > 3 => out.push(format!("wide {margin}")),
        Some(margin) => out.push(format!("narrow {margin}")),
        None => out.push("no margin".to_string()),
    }
    match style.inner {
        Some(Some(inner)) => out.push(format!("inner {inner}")),
        Some(None) => out.push("inner null".to_string()),
        None => out.push("no inner".to_string()),
    }
    let Some(margin) = style.margin else {
        out.push("done".to_string());
        return;
    };
    out.push(format!("last {margin}"));
}

fn main() {
    let mut out = Vec::new();
    describe(parse(r#"{"margin":"4px","inner":"a"}"#), &mut out);
    describe(parse(r#"{"margin":"10px"}"#), &mut out);
    describe(parse("{}"), &mut out);
    for line in out {
        println!("{line}");
    }
}
