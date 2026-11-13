// A `collect()` into an `Rc<[T]>` or an `Arc<[T]>` of a `TrustedLen` asks
// its hint first, as std's `ToRcSlice` does: of no upper bound, `capacity
// overflow`; of a length past what a `Layout` of the slice can be, the
// panic of std's `unwrap()` of its `LayoutError` (#1694), where a `Vec`'s
// panics `capacity overflow`.
use std::panic::{catch_unwind, UnwindSafe};
use std::rc::Rc;
use std::sync::Arc;

fn said(made: impl FnOnce() -> usize + UnwindSafe) -> String {
    match catch_unwind(made) {
        Ok(len) => format!("made {len}"),
        Err(e) => {
            // Either payload's text, by no branch of which it is (the
            // guide's difference 11).
            let text = e.downcast_ref::<String>().map(String::as_str);
            text.or(e.downcast_ref::<&str>().copied()).unwrap_or("?").to_string()
        }
    }
}

// Collected by generic code, whose caller says it's `TrustedLen`.
fn made<C: FromIterator<u64>, I: Iterator<Item = u64>>(items: I) -> C {
    items.collect()
}

fn main() {
    println!(
        "{}",
        said(|| (0..usize::MAX).map(|i| i as u64).collect::<Rc<[u64]>>().len())
    );
    println!(
        "{}",
        said(|| (0..usize::MAX).map(|i| i as u16).collect::<Arc<[u16]>>().len())
    );
    println!("{}", said(|| (0..).map(|i: u64| i).collect::<Rc<[u64]>>().len()));
    println!(
        "{}",
        said(|| (0..usize::MAX).map(|i| i as u64).collect::<Vec<u64>>().len())
    );
    println!("{}", said(|| (0..5u8).map(|i| i * 2).collect::<Rc<[u8]>>().len()));
    println!(
        "{}",
        said(|| made::<Rc<[u64]>, _>((0..usize::MAX).map(|i| i as u64)).len())
    );
    println!("{}", said(|| made::<Arc<[u64]>, _>(0..3u64).len()));
}
