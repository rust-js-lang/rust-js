// A let chain's `if` whose body returns early, React DOM's preload: `if
// (owner != null && href && as) { .. if (..) { // We already have a
// stylesheet .. return; } else if (..) { // .. a script .. return; } .. }`,
// as written, each comment on its own `return`, not the chain turned into
// guards that took the `return`s, `if (!href) { // .. stylesheet ..
// return; }`. The returns the program wrote in a block MIR made are the
// innermost block's, not a block's around it.
fn document(open: bool) -> Option<u32> {
    if open { Some(7) } else { None }
}

fn found(key: &str) -> Option<usize> {
    if key.len() > 4 { Some(key.len()) } else { None }
}

fn preload(log: &mut Vec<String>, href: &str, r#as: &str, open: bool) {
    log.push(format!("preload {href} {as}", as = r#as));
    if let Some(owner) = document(open)
        && !href.is_empty()
        && !r#as.is_empty()
    {
        let key = format!("{href}{owner}");
        if !log.contains(&key) {
            log.push(key.clone());
            if found(&key).is_none() {
                if r#as == "style" && found(href).is_some() {
                    // We already have a stylesheet for this key.
                    return;
                } else if r#as == "script" && found(r#as).is_some() {
                    // We already have a script for this key.
                    return;
                }
                log.push(String::from("made"));
                if r#as == "style" {
                    // Stash a loading state.
                    log.push(String::from("pending"));
                }
                log.push(String::from("hoisted"));
            }
        }
    }
}

// The same chain, two links, its body's `if` of two early returns.
fn shown(log: &mut Vec<String>, href: &str, r#as: &str) {
    if let Some(owner) = document(!href.is_empty())
        && !r#as.is_empty()
    {
        if found(href).is_none() {
            if r#as == "style" {
                // stylesheet
                return;
            } else if r#as == "script" {
                // script
                return;
            }
            log.push(String::from("made"));
        }
        log.push(format!("{owner}"));
    }
}

fn main() {
    let mut log = Vec::new();
    for (href, r#as, open) in [
        ("a", "style", true),
        ("a", "script", true),
        ("abcd", "style", true),
        ("abcd", "script", true),
        ("abcd", "image", true),
        ("", "style", true),
        ("a", "", true),
        ("a", "style", false),
        ("a", "style", true),
    ] {
        preload(&mut log, href, r#as, open);
    }
    for (href, r#as) in [
        ("", "style"),
        ("a", ""),
        ("a", "style"),
        ("a", "script"),
        ("a", "image"),
        ("abcde", "style"),
    ] {
        shown(&mut log, href, r#as);
    }
    println!("{log:?}");
}
