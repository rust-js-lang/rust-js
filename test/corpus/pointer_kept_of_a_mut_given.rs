// A raw pointer a function keeps of a `&mut` it's given writes its caller's
// place after the call (#1781): the `&mut` is given as the place's handle,
// not a box copied back, which the pointer's later write wouldn't reach,
// and so where it's given on to the function that keeps it.
// Miri finds no undefined behaviour: a read through the owner, `n == 0`,
// leaves the pointer it lent able to write.
use std::cell::Cell;

thread_local! {
    static KEPT: Cell<*mut u32> = const { Cell::new(std::ptr::null_mut()) };
}

fn keep(n: &mut u32) {
    KEPT.with(|kept| kept.set(n));
}

fn zero_kept() {
    KEPT.with(|kept| unsafe { *kept.get() = 0 });
}

#[inline(never)]
fn apply<F: FnOnce()>(f: F) {
    f();
}

fn kept_in_a_closure(n: &mut u32) {
    apply(move || KEPT.with(|kept| kept.set(n)));
}

trait GetMut {
    fn get_mut(&mut self) -> &mut u32;
}

impl GetMut for u32 {
    fn get_mut(&mut self) -> &mut u32 {
        self
    }
}

fn kept_by_generic_code(n: &mut impl GetMut) {
    let n = n.get_mut();
    KEPT.with(|kept| kept.set(n));
}

fn passed_on(n: &mut u32) {
    keep(n);
}

fn add_one(n: &mut u32) {
    *n += 1;
}

fn stashed(mut n: u32) -> u32 {
    keep(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_by_another(mut n: u32) -> u32 {
    passed_on(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_in_a_closure(mut n: u32) -> u32 {
    kept_in_a_closure(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_by_generic_code(mut n: u32) -> u32 {
    kept_by_generic_code(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn bumped(mut n: u32) -> u32 {
    add_one(&mut n);
    n
}

fn main() {
    println!("{}", stashed(5));
    println!("{}", stashed_by_another(7));
    println!("{}", stashed_in_a_closure(9));
    println!("{}", stashed_by_generic_code(11));
    println!("{}", bumped(5));
}
