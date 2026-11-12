//! A name's case as JS writes it (ADRs 0038, 0573): a Rust name camelCase,
//! a thread-local's `SCREAMING_SNAKE` one too, a hyphenated key's variable,
//! a module's namespace, a word's first letter. Split from lower.rs by
//! concept, as a file is at most 1500 lines.

/// `name` without its hyphens, each next letter capitalized: a hyphenated
/// key's variable, `data-step`'s `dataStep`.
pub(in crate::lower) fn unhyphenated(name: &str) -> String {
    let mut out = String::new();
    let mut capital = false;
    for c in name.chars() {
        match (c, capital) {
            ('-', _) => capital = true,
            (c, true) => {
                out.extend(c.to_uppercase());
                capital = false;
            }
            (c, false) => out.push(c),
        }
    }
    out
}

/// A Rust name as JS code writes it (ADRs 0038, 0573): `set_count` is
/// `setCount`. Leading and trailing underscores stay (`_unused`, `type_`),
/// and so does a name with no lowercase letter, a component's `H2` say.
/// The one place a `snake_case` name is made camelCase; a thread-local's
/// `SCREAMING_SNAKE` one is `variable_case`'s.
pub(in crate::lower) fn camel_case(name: &str) -> String {
    let core = name.trim_matches('_');
    if !core.contains('_') || !core.contains(|c: char| c.is_ascii_lowercase()) {
        return name.to_string();
    }
    words_joined(name, |word| word.to_string())
}

/// A module's namespace as JS names it where it's imported whole, of its
/// JS file's stem or its Rust name: its words camelCase, its first letter
/// a capital, as JS names a namespace after its module's file (rd114b/c,
/// ADR 1018). `DOMPluginEventSystem` is its file's, `EnterLeaveEventPlugin`
/// of `enter_leave_event_plugin`, and a binding's namespace is named so
/// too, `Scheduler` of `scheduler` (ADR 0256): the one rule of its name.
pub(in crate::lower) fn namespace_name(name: &str) -> String {
    upper_first(&crate::lower::bindings::stem_binding(name))
}

/// A thread-local's name as JS writes its module's variable (ADR 0573):
/// its `SCREAMING_SNAKE` words lowercase, then camelCase, `TASK_QUEUE` as
/// `taskQueue`; another, as `camel_case` writes it.
pub(in crate::lower) fn variable_case(name: &str) -> String {
    let core = name.trim_matches('_');
    if core.contains(|c: char| c.is_ascii_lowercase()) || !core.contains(|c: char| c.is_ascii_uppercase()) {
        return camel_case(name);
    }
    words_joined(name, |word| word.to_ascii_lowercase())
}

/// `name`'s words, each `cased`, joined as camelCase, between its leading
/// and trailing underscores.
fn words_joined(name: &str, cased: impl Fn(&str) -> String) -> String {
    let core = name.trim_matches('_');
    let lead = &name[..name.len() - name.trim_start_matches('_').len()];
    let trail = &name[name.trim_end_matches('_').len()..];
    let mut out = lead.to_string();
    for (i, word) in core.split('_').filter(|w| !w.is_empty()).enumerate() {
        match i {
            0 => out.push_str(&cased(word)),
            _ => out.push_str(&upper_first(&cased(word))),
        }
    }
    out.push_str(trail);
    out
}

/// `counter` → `Counter`: a word joined into a camelCase name, after the
/// first, `RefFlags` of `Ref` and `flags`.
pub(crate) fn upper_first(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|c| std::iter::once(c.to_ascii_uppercase()).chain(chars).collect())
        .unwrap_or_default()
}

/// `Counter` → `counter`: a value of the type, named after it.
pub(crate) fn lower_first(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map(|c| c.to_lowercase().chain(chars).collect())
        .unwrap_or_default()
}
