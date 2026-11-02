// A closure's shared borrow of a field of a struct JS made, which a getter
// may answer, is read where the closure derefs it, as Rust reads it there:
// `run(() => describe(entry.value) + describe(entry.value))`, not a
// `const captured = entry.value;` its maker read first (rd631, ADR 1022,
// amended, the user's decision). One the closure gives on as a reference
// is read where it's made, as before. Natively, the binding's body makes
// what JS's `JSON.parse` gives.

struct Entry {
    done: bool,
    value: Option<&'static str>,
}

#[cfg_attr(rust_js, rust_js::link_name = "JSON.parse")]
fn parse(text: &str) -> Entry {
    let value = text.find("\"value\":\"").map(|at| {
        let start = at + "\"value\":\"".len();
        let end = start + text[start..].find('"').unwrap_or(0);
        &*Box::leak(text[start..end].to_string().into_boxed_str())
    });
    Entry {
        done: text.contains("\"done\":true"),
        value,
    }
}

fn run<R>(f: impl FnOnce() -> R) -> R {
    f()
}

fn describe(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("<{value}>"),
        None => "none".to_string(),
    }
}

fn described(value: &Option<&str>) -> String {
    describe(*value)
}

fn part(text: &str) -> String {
    let entry = parse(text);
    if entry.done {
        return "done".to_string();
    }
    let twice = run(|| describe(entry.value) + &describe(entry.value));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let first = describe(entry.value);
        first + &describe(entry.value).to_uppercase()
    }));
    let given = run(|| described(&entry.value));
    format!("{twice} {} {given}", caught.unwrap_or_default())
}

fn main() {
    println!("{}", part(r#"{"done":false,"value":"a"}"#));
    println!("{}", part(r#"{"done":false}"#));
    println!("{}", part(r#"{"done":true,"value":"b"}"#));
}
