// A getter a binding reads, `get kind`, read once for each call Rust
// makes of it, however its value is tested: a `matches!` of
// alternatives in a chain (#485), a `match` with guards, `||` and `&&`
// chains, a let chain, and a value read before a call with effects, in
// Rust's order (#564).
//@ call: capable ["input", {"kind": 1}, false]
//@ call: capable ["input", {"kind": 3}, false]
//@ call: capable ["textarea", {"kind": 1}, true]
//@ call: described [{"kind": 4, "name": "n"}]
//@ call: described [{"kind": 1, "name": "n"}]
//@ call: described [{"kind": 9, "name": "n"}]
//@ call: either [{"kind": 2, "name": ""}]
//@ call: either [{"kind": 0, "name": "x"}]
//@ call: ordered [{"kind": 2, "name": "a", "touch": {"$fn": 7}}]
//@ call: chained [{"kind": 2, "name": "abc"}]
//@ call: chained [{"kind": 5, "name": ""}]

pub trait Input {
    #[cfg_attr(rust_js, rust_js::link_name = "get kind")]
    fn kind(&self) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get name")]
    fn name(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "touch")]
    fn touch(&self) -> u32 {
        unreachable!()
    }
}

pub struct Element;

impl Input for Element {}

pub fn capable(name: &str, input: &Element, editable: bool) -> bool {
    !name.is_empty() && ((name == "input" && matches!(input.kind(), 2 | 1)) || name == "textarea" || editable)
}

pub fn described(input: &Element) -> String {
    match input.kind() {
        kind if kind > 3 => format!("big {kind}"),
        1 | 2 => input.name(),
        kind => format!("{kind} {}", input.name()),
    }
}

pub fn either(input: &Element) -> bool {
    input.kind() == 2 || input.name().is_empty() || input.kind() > 1 && input.name() == "x"
}

pub fn ordered(input: &Element) -> String {
    format!("{} {} {}", input.kind(), input.touch(), input.name())
}

pub fn chained(input: &Element) -> usize {
    if let kind @ 1..=3 = input.kind()
        && let name = input.name()
        && !name.is_empty()
    {
        name.len() + kind as usize
    } else {
        0
    }
}
