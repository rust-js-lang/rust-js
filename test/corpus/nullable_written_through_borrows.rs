// A `#[rust_js::nullable]` place stores `null` for `None` however it's
// written, so its reads, which a nullable parameter tests `=== null`, are
// right (#541): a thread-local through what `with` and `with_borrow_mut`
// lend, `c.set(None)`, `*t = None`, `take`, `replace`, `mem::replace`,
// `mem::swap` and generic code given the `&mut`, of one that's its module's
// variable, `current = noneOf(1) ?? null`, and of one whose borrows are
// counted, a cell that stores so itself, `$nullableCell(null)`; a field,
// and a function's own nullable parameter, through a `&mut` a callee's box
// is copied back from, `fiber.owner = slot.value ?? null`, and the same of
// an item of a nullable field's items and of a nullable `RefCell` field's
// `get_mut()`.
use std::cell::{Cell, RefCell};

thread_local! {
    // Lent only where nothing else borrows them: their module's variables.
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURRENT: Cell<Option<u32>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static TARGET: RefCell<Option<u32>> = const { RefCell::new(None) };
    // Lent by `with` and borrowed there: a cell that counts its borrows.
    #[cfg_attr(rust_js, rust_js::nullable)]
    static COUNTED: RefCell<Option<u32>> = const { RefCell::new(None) };
}

fn none_of(n: u32) -> Option<u32> {
    if n > 3 { Some(n) } else { None }
}

struct Fiber {
    #[cfg_attr(rust_js, rust_js::nullable)]
    owner: Option<u32>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    lanes: Vec<Option<u32>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    target: RefCell<Option<u32>>,
}

#[cfg_attr(rust_js, rust_js::nullable(x))]
fn absent(x: Option<u32>) -> bool {
    x.is_none()
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn current() -> Option<u32> {
    CURRENT.get()
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn counted() -> Option<u32> {
    COUNTED.with_borrow(Clone::clone)
}

fn clear_slot(slot: &mut Option<u32>, n: u32) {
    *slot = none_of(n);
}

fn clear_generic<T>(slot: &mut Option<T>) {
    *slot = None;
}

fn swap_with<T>(slot: &mut Option<T>, other: &mut Option<T>) {
    std::mem::swap(slot, other);
}

fn clear_cell(cell: &Cell<Option<u32>>) {
    cell.set(none_of(1));
}

#[cfg_attr(rust_js, rust_js::nullable(x))]
fn param_cleared(mut x: Option<u32>) -> bool {
    clear_generic(&mut x);
    absent(x)
}

#[cfg_attr(rust_js, rust_js::nullable(x))]
fn param_swapped(mut x: Option<u32>) -> bool {
    let mut other = none_of(1);
    std::mem::swap(&mut x, &mut other);
    absent(x)
}

fn reset() {
    CURRENT.set(Some(4));
    TARGET.set(Some(4));
    COUNTED.set(Some(4));
}

fn report(what: &str) {
    let current_is = if let Some(c) = CURRENT.get() { c } else { 0 };
    println!(
        "{what}: {} {} {} {:?} {:?} {current_is} {} {}",
        absent(CURRENT.get()),
        absent(TARGET.with_borrow(Clone::clone)),
        absent(COUNTED.with_borrow(Clone::clone)),
        current(),
        counted(),
        CURRENT.get().is_none(),
        TARGET.with_borrow(Option::is_none),
    );
}

fn report_fiber(what: &str, fiber: &Fiber) {
    println!(
        "{what}: {} {} {:?} {} {:?} {}",
        absent(fiber.owner),
        fiber.owner.is_none(),
        fiber.owner,
        fiber.lanes[0].is_none(),
        fiber.target.borrow(),
        fiber.target.borrow().is_none(),
    );
}

fn main() {
    reset();
    CURRENT.with(|c| c.set(none_of(1)));
    TARGET.with(|t| *t.borrow_mut() = none_of(1));
    COUNTED.with(|t| {
        let guard = t.borrow();
        drop(guard);
        *t.borrow_mut() = none_of(1);
    });
    report("with, set");

    reset();
    CURRENT.with(clear_cell);
    report("with, given on");

    reset();
    TARGET.with_borrow_mut(|t| *t = none_of(1));
    COUNTED.with_borrow_mut(|t| *t = none_of(1));
    report("with_borrow_mut, assigned");

    reset();
    CURRENT.with(|c| c.take());
    TARGET.with_borrow_mut(|t| t.take());
    COUNTED.with(|t| t.take());
    report("take");

    reset();
    CURRENT.with(|c| c.replace(none_of(2)));
    COUNTED.with(|t| t.replace(none_of(2)));
    report("replace");

    reset();
    TARGET.with_borrow_mut(|t| std::mem::replace(t, none_of(2)));
    COUNTED.with_borrow_mut(|t| std::mem::replace(t, none_of(2)));
    report("mem::replace");

    reset();
    let mut other = None;
    TARGET.with_borrow_mut(|t| std::mem::swap(t, &mut other));
    let mut other = None;
    COUNTED.with_borrow_mut(|t| std::mem::swap(t, &mut other));
    report("mem::swap");

    reset();
    TARGET.with_borrow_mut(|t| clear_generic(t));
    COUNTED.with_borrow_mut(clear_generic);
    report("generic");

    reset();
    let mut other = none_of(0);
    TARGET.with_borrow_mut(|t| swap_with(t, &mut other));
    report("generic swap");

    reset();
    TARGET.with_borrow_mut(|t| clear_slot(t, 1));
    COUNTED.with_borrow_mut(|t| clear_slot(t, 1));
    report("given on");

    reset();
    CURRENT.with(|c| {
        c.set(none_of(1));
        println!("inside: {} {:?}", absent(CURRENT.get()), current());
    });

    let mut fiber = Fiber {
        owner: Some(3),
        lanes: vec![Some(1)],
        target: RefCell::new(Some(3)),
    };
    report_fiber("made", &fiber);
    clear_slot(&mut fiber.owner, 1);
    clear_generic(&mut fiber.lanes[0]);
    clear_generic(fiber.target.get_mut());
    report_fiber("given on", &fiber);

    fiber.owner = Some(3);
    let mut other = none_of(1);
    std::mem::swap(&mut fiber.owner, &mut other);
    report_fiber("mem::swap", &fiber);

    fiber.owner = Some(3);
    let mut other = none_of(1);
    swap_with(&mut fiber.owner, &mut other);
    report_fiber("generic swap", &fiber);

    fiber.owner = Some(3);
    let slot = &mut fiber.owner;
    *slot = none_of(1);
    report_fiber("through a let", &fiber);

    println!("parameters: {} {}", param_cleared(Some(3)), param_swapped(Some(3)));
}
