// The `bool` a `matches!` of an `Option` sets in each arm of its `match`,
// or a variable of it or its negation, proves the `Option` on its edge:
// returned from a nullable function as it is; past the `if`, given as
// `null` where it's `None` (#1933 step 4 (D)).
#[cfg_attr(rust_js, rust_js::nullable)]
fn digit(items: &[u32]) -> Option<u32> {
    let found = items.first().copied();
    if matches!(found, Some(1..=9)) {
        return found;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn nonzero(items: &[u32]) -> Option<u32> {
    let found = items.last().copied();
    let kept = matches!(found, Some(1..));
    if kept {
        return found;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn past(items: &[u32], strict: bool) -> Option<u32> {
    let found = items.first().copied();
    if strict && matches!(found, Some(1..=9)) {
        return Some(0);
    }
    found
}

fn main() {
    println!("{:?} {:?} {:?}", digit(&[3]), digit(&[12]), digit(&[]));
    println!("{:?} {:?} {:?}", nonzero(&[1, 5]), nonzero(&[0]), nonzero(&[]));
    println!("{:?} {:?} {:?}", past(&[4], true), past(&[4], false), past(&[], true));
}
