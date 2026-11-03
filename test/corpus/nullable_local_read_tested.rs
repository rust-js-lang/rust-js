// A `#[rust_js::nullable]` thread-local's whole read, `get()`,
// `with_borrow(Clone::clone)`, `take()` or `replace(..)`, is `null` or a
// value: never `undefined`, but maybe `None`. An `Option` made of it where
// it's tested, `keep.then(|| CURRENT.get())`, is there only where the read
// is too, so its presence is tested, not taken from `keep` alone (#539);
// so is a nullable function's result, `keep.then(|| found(n))`.
use std::cell::{Cell, RefCell};

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURRENT: Cell<Option<u32>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static TARGET: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn found(n: u32) -> Option<u32> {
    if n > 3 { Some(n) } else { None }
}

fn has_current(keep: bool) -> bool {
    keep.then(|| CURRENT.get()).flatten().is_some()
}

fn has_target(keep: bool) -> bool {
    keep.then(|| TARGET.with_borrow(Clone::clone)).flatten().is_some()
}

fn lacks_current(keep: bool) -> bool {
    keep.then(|| CURRENT.get()).flatten().is_none()
}

fn has_current_given(keep: bool) -> bool {
    keep.then_some(CURRENT.get()).flatten().is_some()
}

fn current_or(keep: bool) -> u32 {
    if let Some(current) = keep.then(|| CURRENT.get()).flatten() {
        current
    } else {
        99
    }
}

fn target_len(keep: bool) -> Option<usize> {
    keep.then(|| TARGET.with_borrow(Clone::clone))
        .flatten()
        .map(|target| target.len())
}

fn current_unwrapped(keep: bool) -> u32 {
    keep.then(|| CURRENT.get()).flatten().unwrap_or(7)
}

fn mapped_current(given: Option<u32>) -> bool {
    given.map(|_| CURRENT.get()).flatten().is_some()
}

fn took_target(keep: bool) -> bool {
    keep.then(|| TARGET.take()).flatten().is_some()
}

fn replaced_current(keep: bool) -> bool {
    keep.then(|| CURRENT.replace(None)).flatten().is_some()
}

fn has_found(keep: bool, n: u32) -> bool {
    keep.then(|| found(n)).flatten().is_some()
}

fn set(present: bool) {
    CURRENT.set(present.then_some(5));
    TARGET.set(present.then(|| vec!["a".to_string(), "b".to_string()]));
}

fn main() {
    for present in [false, true] {
        for keep in [false, true] {
            set(present);
            println!(
                "{present} {keep}: {} {} {} {} {} {:?} {} {}",
                has_current(keep),
                has_target(keep),
                lacks_current(keep),
                has_current_given(keep),
                current_or(keep),
                target_len(keep),
                current_unwrapped(keep),
                mapped_current(keep.then_some(1)),
            );
            println!(
                "  taken {} replaced {} found {}",
                took_target(keep),
                replaced_current(keep),
                has_found(keep, if present { 5 } else { 1 }),
            );
            println!(
                "  after {} {}",
                CURRENT.get().is_some(),
                TARGET.with_borrow(Option::is_some)
            );
        }
    }
}
