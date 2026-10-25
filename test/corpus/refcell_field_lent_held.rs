//@ run-fail: RefCell already borrowed
// A struct's `RefCell` field lent to a function is its one cell there,
// `$fieldCell(wrap, "n")`, which counts its borrows: one held through the
// reference while the field is borrowed mutably finds it borrowed, as
// std's does (ADR 0863, amended).
use std::cell::RefCell;

struct Wrap {
    n: RefCell<u16>,
}

fn hold(cell: &RefCell<u16>, f: impl Fn()) {
    let held = cell.borrow();
    f();
    println!("{}", *held);
}

fn bump(wrap: &Wrap) {
    let n = *wrap.n.borrow();
    wrap.n.replace(n + 1);
}

fn main() {
    let wrap = Wrap { n: RefCell::new(1) };
    bump(&wrap);
    hold(&wrap.n, || {});
    hold(&wrap.n, || bump(&wrap));
}
