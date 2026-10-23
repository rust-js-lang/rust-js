// An offset only a place in the string it was found in counts JS's units:
// `find` is `indexOf`, a slice at it `slice`, `strip_prefix` `startsWith`
// (ADR 0642). Each on text of one, two, three and four bytes a char.

// React's owner stack, as `shared/ReactOwnerStackFrames.js` takes it apart.
fn format_owner_stack(stack: &str) -> String {
    let mut stack = stack;
    if let Some(rest) = stack.strip_prefix("Error: react-stack-top-frame\n") {
        stack = rest;
    }
    if let Some(idx) = stack.find('\n') {
        // Pop the JSX frame.
        stack = &stack[idx + 1..];
    }
    let idx = stack
        .find("react_stack_bottom_frame")
        .and_then(|idx| stack[..idx].rfind('\n'));
    if let Some(idx) = idx {
        stack = &stack[..idx];
    } else {
        return String::new();
    }
    stack.to_string()
}

fn first_word(s: &str) -> &str {
    match s.find(' ') {
        Some(at) => &s[..at],
        None => s,
    }
}

fn after<'a>(s: &'a str, pattern: &str) -> &'a str {
    match s.find(pattern) {
        Some(at) => &s[at + pattern.len()..],
        None => "",
    }
}

fn after_char(s: &str, c: char) -> &str {
    match s.rfind(c) {
        Some(at) => &s[at + c.len_utf8()..],
        None => s,
    }
}

fn pieces(s: &str) -> Vec<&str> {
    let mut rest = s;
    let mut pieces = Vec::new();
    while let Some(at) = rest.find(',') {
        pieces.push(&rest[..at]);
        rest = &rest[at + 1..];
    }
    pieces.push(rest);
    pieces
}

fn between(s: &str) -> &str {
    if let Some(open) = s.find("«")
        && let Some(close) = s.rfind("»")
        && open < close
    {
        return &s[open + "«".len()..s.len()];
    }
    s
}

fn extension(name: &str) -> Option<&str> {
    let dot = name.rfind('.')?;
    Some(&name[dot + 1..])
}

fn trimmed(s: &str) -> &str {
    let s = s.strip_prefix("é-").unwrap_or(s);
    if let Some(rest) = s.strip_suffix("🦀") {
        return rest;
    }
    s
}

fn strip_held<'a>(s: &'a str, prefix: &str, suffix: char) -> &'a str {
    let mut s = s;
    if let Some(rest) = s.strip_prefix(prefix) {
        s = rest;
    }
    if let Some(rest) = s.strip_suffix(suffix) {
        s = rest;
    }
    s
}

fn main() {
    let stacks = [
        "Error: react-stack-top-frame\n    at jsx\n    at App (app.js:1)\n    at Main (é.js:2)\n    at react_stack_bottom_frame (r.js:3)",
        "Error: react-stack-top-frame\n    at jsx\n    at 🦀 (crab.js:1)\n    at react_stack_bottom_frame",
        "no frames here",
        "Error: other\n    at jsx\n    at react_stack_bottom_frame",
    ];
    for stack in stacks {
        println!("[{}]", format_owner_stack(stack));
    }
    for s in ["hello world", "héllo wörld", "日本 語", "🦀 crab", "", "none"] {
        println!("{} | {} | {} | {}", first_word(s), after(s, "l"), after(s, "ö"), after_char(s, 'o'));
        println!("{:?} {:?}", pieces(s), after_char(s, '🦀'));
    }
    for s in ["a,é,🦀,", ",,", "日,本"] {
        println!("{:?}", pieces(s));
    }
    for s in ["x«é»y", "«🦀»", "»«", "plain"] {
        println!("{}", between(s));
    }
    for name in ["a.rs", "é.tar.gz", "🦀.é", "none"] {
        println!("{:?}", extension(name));
    }
    for s in ["é-word🦀", "é-🦀", "🦀", "word"] {
        println!("{}", trimmed(s));
    }
    println!("{} {}", strip_held("préfixe-🦀x", "préfixe-", 'x'), strip_held("abc", "", 'é'));
}
