// An `Option` of an `Option` held where it's made, its payload (ADR 1017),
// tested by std's methods and by `==` with `Some(None)`: `last !==
// undefined`, `last === undefined` and `last === null`, and `if let
// Some(Some(v))`'s `last != null`, not `$eq(last, { $someNone: 0 })`,
// which kept it boxed (ADR 1017, amended). Each of the three values, and
// none written, against native Rust.
fn last_of(items: &[Option<u32>]) -> String {
    let mut last = None;
    for item in items {
        last = Some(*item);
    }
    let mut out = String::new();
    if last.is_some() {
        out.push('s');
    }
    if last.is_none() {
        out.push('n');
    }
    if last == Some(None) {
        out.push('0');
    }
    if last != Some(None) {
        out.push('x');
    }
    if let Some(Some(v)) = last {
        out.push_str(&v.to_string());
    }
    out
}

// Of `Some(Some(2))`, no `Some(None)`: its box, compared as it is.
fn last_is_two(items: &[Option<u32>]) -> bool {
    let mut last = None;
    for item in items {
        last = Some(*item);
    }
    last == Some(Some(2))
}

fn main() {
    for items in [&[Some(1), Some(2)][..], &[Some(1), None], &[]] {
        println!("{} {}", last_of(items), last_is_two(items));
    }
}
