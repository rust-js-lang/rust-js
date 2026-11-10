// A parameter the body sets again, by a call of a generic function, of
// one that may give `None`, or to `None`, holds what its caller gave
// until then, `undefined` for `None`: given on to the crate's own
// `#[rust_js::nullable]` parameter, which then tests it `!= null`, never
// `!== null` (ADR 0781, amended, #1248). Natively, the same answers.
#[cfg_attr(rust_js, rust_js::nullable(reason))]
fn commit(reason: Option<&str>) -> usize {
    match reason {
        Some(reason) => reason.len() + 1,
        None => 0,
    }
}

fn some<T>(x: T) -> Option<T> {
    Some(x)
}

fn plain(set: bool) -> Option<&'static str> {
    if set { Some("ab") } else { None }
}

pub fn through_generic(mut reason: Option<&'static str>, set: bool) -> usize {
    if set {
        reason = some("a");
    }
    commit(reason)
}

pub fn through_plain(mut reason: Option<&'static str>, set: bool) -> usize {
    if set {
        reason = plain(true);
    }
    commit(reason)
}

pub fn none_or_call(mut reason: Option<&'static str>, set: bool) -> usize {
    if set {
        reason = plain(false);
    } else if reason.is_some() {
        reason = None;
    }
    commit(reason)
}

fn main() {
    println!("{}", through_generic(None, false));
    println!("{}", through_generic(None, true));
    println!("{}", through_generic(Some("xyz"), false));
    println!("{}", through_plain(None, false));
    println!("{}", through_plain(None, true));
    println!("{}", none_or_call(None, false));
    println!("{}", none_or_call(Some("q"), false));
    println!("{}", none_or_call(None, true));
}
