// `take(n)` of an iterator whose upper bound is unknown, a binding's JS
// iterator behind a `Box<dyn Iterator>` or a `from_fn`'s, asked its hint by
// generic code: std's `Take` bounds it by `n`, `(0, Some(n))`, though its
// source says none (#1691).
fn hint_of<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

/// `items.values()`: JS's own iterator of an array, as a binding gives
/// one; natively, an iterator of the same items that knows no hint.
#[cfg_attr(rust_js, rust_js::link_name = "values")]
fn js_values(this: &Vec<u32>) -> Box<dyn Iterator<Item = u32>> {
    #[cfg(not(rust_js))]
    return {
        let mut items = this.clone().into_iter();
        Box::new(std::iter::from_fn(move || items.next()))
    };
    #[cfg(rust_js)]
    unreachable!()
}

fn unknown() -> Box<dyn Iterator<Item = u32>> {
    let mut n = 0;
    Box::new(std::iter::from_fn(move || {
        n += 1;
        (n <= 5).then_some(n)
    }))
}

fn main() {
    let nums = vec![1, 2, 3, 4, 5];
    println!("{:?}", hint_of(js_values(&nums).take(3)));
    println!("{:?}", hint_of(js_values(&nums).take(0)));
    println!("{:?}", hint_of(unknown().take(2)));
    let taken: Vec<u32> = js_values(&nums).take(3).collect();
    println!("{taken:?}");
}
