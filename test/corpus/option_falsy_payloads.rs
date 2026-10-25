// An `Option` whose `Some` may be falsy in JS, `0`, `NaN`, `""`, `0n`,
// `false`, a number's, text's, a bigint's or a value of any shape's, is
// there where it's not `null` nor `undefined`, `o != null`; one of an
// object, an array or a function, which is never falsy, where it's
// truthy, `o` (ADR 0298, amended). Each takes the branch Rust's does,
// `Some(0)` and `Some("")` as `Some`, `None` as `None`.

use std::marker::PhantomData;

/// A JS value of any shape, as `js::Unknown` is: whatever `JSON.parse`
/// gives, `0`, `""` and `false` too, never `null` nor `undefined`, which an
/// `Option` of one is `None` of.
#[cfg_attr(rust_js, rust_js::types = "unknown")]
struct Value(PhantomData<*mut ()>, u8);

/// `JSON.parse(text)`: `null` of `"null"`, a `None`, and the value of any
/// other, `0`, `""`, `false`, an object.
#[cfg_attr(rust_js, rust_js::link_name = "JSON.parse")]
fn parse(text: &str) -> Option<&'static Value> {
    match text {
        "null" => None,
        _ => Some(Box::leak(Box::new(Value(PhantomData, 0)))),
    }
}

/// `Number(text)`: `NaN` of text that's no number.
#[cfg_attr(rust_js, rust_js::link_name = "Number")]
fn number(text: &str) -> Option<&'static Value> {
    let _ = text;
    Some(Box::leak(Box::new(Value(PhantomData, 0))))
}

/// `BigInt(text)`: `0n` of `"0"`.
#[cfg_attr(rust_js, rust_js::link_name = "BigInt")]
fn bigint(text: &str) -> Option<&'static Value> {
    let _ = text;
    Some(Box::leak(Box::new(Value(PhantomData, 0))))
}

struct Point {
    x: i32,
}

fn has_value(value: Option<&Value>) -> bool {
    value.is_some()
}

fn lacks_value(value: Option<&Value>) -> bool {
    value.is_none()
}

fn value_branch(value: Option<&Value>) -> &'static str {
    if let Some(_v) = value { "some" } else { "none" }
}

fn value_match(value: Option<&Value>) -> &'static str {
    match value {
        Some(_) => "some",
        None => "none",
    }
}

fn value_tried(value: Option<&Value>) -> Option<u32> {
    let _v = value?;
    Some(1)
}

fn count_values(values: &[Option<&Value>]) -> usize {
    values.iter().filter_map(|v| *v).count()
}

fn count_some<T: Copy>(values: &[Option<T>]) -> usize {
    values.iter().filter(|v| v.is_some()).count()
}

fn number_branch(n: Option<u32>) -> &'static str {
    if let Some(_n) = n { "some" } else { "none" }
}

fn float_branch(n: Option<f64>) -> &'static str {
    if n.is_none() { "none" } else { "some" }
}

fn big_branch(n: Option<i64>) -> &'static str {
    match n {
        Some(_) => "some",
        None => "none",
    }
}

fn text_branch(s: Option<&str>) -> &'static str {
    if let Some(s) = s {
        if s.is_empty() { "empty" } else { "text" }
    } else {
        "none"
    }
}

fn string_tried(s: Option<String>) -> Option<usize> {
    let s = s?;
    Some(s.len() + 1)
}

fn flag_branch(f: Option<bool>) -> &'static str {
    if let Some(f) = f {
        if f { "true" } else { "false" }
    } else {
        "none"
    }
}

fn flag_present(f: Option<bool>) -> bool {
    f.is_some()
}

fn point_branch(p: Option<&Point>) -> &'static str {
    if let Some(p) = p {
        if p.x == 0 { "origin" } else { "point" }
    } else {
        "none"
    }
}

fn list_branch(list: Option<Vec<u32>>) -> &'static str {
    match list {
        Some(list) if list.is_empty() => "empty",
        Some(_) => "list",
        None => "none",
    }
}

fn called(f: Option<&dyn Fn() -> u32>) -> u32 {
    if let Some(f) = f { f() } else { 7 }
}

fn first_point(points: &[Option<&Point>]) -> Option<i32> {
    let p = points.first()?;
    let p = (*p)?;
    Some(p.x)
}

fn popped(mut stack: Vec<u32>) -> u32 {
    let mut sum = 0;
    let mut n = 0;
    while let Some(x) = stack.pop() {
        sum += x;
        n += 1;
    }
    sum * 10 + n
}

fn popped_text(mut stack: Vec<&str>) -> usize {
    let mut n = 0;
    while let Some(s) = stack.pop() {
        n += s.len() + 1;
    }
    n
}

fn popped_points(mut stack: Vec<Point>) -> i32 {
    let mut n = 0;
    while let Some(p) = stack.pop() {
        n += p.x + 1;
    }
    n
}

fn main() {
    // Values of any shape: `0`, `""`, `false`, `NaN` and `0n` are there.
    let values = [
        parse("0"),
        parse("\"\""),
        parse("false"),
        number("x"),
        bigint("0"),
        parse("{}"),
        parse("null"),
    ];
    for value in values {
        println!(
            "{} {} {} {} {:?}",
            has_value(value),
            lacks_value(value),
            value_branch(value),
            value_match(value),
            value_tried(value)
        );
    }
    println!("{}", count_values(&values));
    // Numbers, text and bigints that are falsy are there too.
    println!(
        "{} {} {}",
        number_branch(Some(0)),
        number_branch(Some(1)),
        number_branch(None)
    );
    println!(
        "{} {} {} {}",
        float_branch(Some(0.0)),
        float_branch(Some(-0.0)),
        float_branch(Some(f64::NAN)),
        float_branch(None)
    );
    println!("{} {}", big_branch(Some(0)), big_branch(None));
    println!(
        "{} {} {}",
        text_branch(Some("")),
        text_branch(Some("a")),
        text_branch(None)
    );
    println!("{:?} {:?}", string_tried(Some(String::new())), string_tried(None));
    println!(
        "{} {} {}",
        flag_branch(Some(false)),
        flag_branch(Some(true)),
        flag_branch(None)
    );
    println!("{} {}", flag_present(Some(false)), flag_present(None));
    println!(
        "{} {} {}",
        count_some(&[Some(0), None, Some(0)]),
        count_some(&[Some(""), None]),
        count_some(&[Some(false), Some(true), None])
    );
    // Objects, arrays and functions: never falsy, an empty one too.
    let origin = Point { x: 0 };
    println!(
        "{} {} {}",
        point_branch(Some(&origin)),
        point_branch(Some(&Point { x: 2 })),
        point_branch(None)
    );
    println!(
        "{} {} {}",
        list_branch(Some(vec![])),
        list_branch(Some(vec![0])),
        list_branch(None)
    );
    let zero = || 0;
    println!("{} {}", called(Some(&zero)), called(None));
    println!(
        "{:?} {:?} {:?}",
        first_point(&[Some(&origin)]),
        first_point(&[None]),
        first_point(&[])
    );
    // `while let` of what `pop` gives: `0` and `""` are popped too.
    println!(
        "{} {} {}",
        popped(vec![0, 0, 3]),
        popped_text(vec!["", "ab", ""]),
        popped_points(vec![Point { x: 0 }, Point { x: 0 }])
    );
}
