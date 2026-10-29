// A struct JS made, a binding's result, is a JS object the program didn't
// build, whose properties a getter may answer: each read of a field of it
// is JS's, as often as Rust reads it, so a `let` of one keeps its `const`
// (rd204, ADR 1022). One the program builds is plain data, read in place.
// Natively, the binding's body makes what JS's `JSON.parse` gives.

struct Style {
    margin: Option<&'static str>,
    padding: Option<&'static str>,
}

#[cfg_attr(rust_js, rust_js::link_name = "JSON.parse")]
fn parse(text: &str) -> &'static Style {
    let field = |name: &str| {
        let key = format!("\"{name}\":\"");
        let at = text.find(&key)? + key.len();
        let end = text[at..].find('"')?;
        Some(&*Box::leak(text[at..at + end].to_string().into_boxed_str()))
    };
    Box::leak(Box::new(Style {
        margin: field("margin"),
        padding: field("padding"),
    }))
}

fn describe(style: &Style, out: &mut Vec<String>) {
    let margin = style.margin;
    if margin.is_some() {
        out.push(format!("margin {}", margin.unwrap()));
    } else {
        let padding = style.padding;
        out.push(format!("padding {padding:?}"));
    }
}

fn main() {
    let mut out = Vec::new();
    describe(parse(r#"{"margin":"4px"}"#), &mut out);
    describe(parse(r#"{"padding":"2px"}"#), &mut out);
    describe(parse("{}"), &mut out);
    let own = Style {
        margin: Some("1px"),
        padding: None,
    };
    describe(&own, &mut out);
    for line in out {
        println!("{line}");
    }
}
