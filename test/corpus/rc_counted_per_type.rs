// Each type's `Rc`s are counted apart (ADR 0852): an `Rc<Cell<f64>>` whose
// count is read is `{ value, strong, weak }`, an `Rc<Cell<u32>>` beside it
// the value itself; and a generic function's `Rc<Slot<T>>` is every
// `Rc<Slot<_>>`, counted with the ones whose counts are read. Generic code's
// `Rc<Pair<T, u8>>`, which may be a counted `Rc<Pair<bool, u8>>`, is counted,
// and so is each it may be given, an `Rc<Pair<f64, u8>>` no count is read of.
use std::cell::Cell;
use std::rc::Rc;

struct Slot<T> {
    value: T,
}

fn pair<T>(value: T) -> (Rc<Slot<T>>, Rc<Slot<T>>) {
    let slot = Rc::new(Slot { value });
    (Rc::clone(&slot), slot)
}

struct Pair<A, B> {
    first: A,
    second: B,
}

fn again<T>(pair: &Rc<Pair<T, u8>>) -> Rc<Pair<T, u8>> {
    Rc::clone(pair)
}

fn main() {
    let hits = Rc::new(Cell::new(0.5));
    let again_hits = Rc::clone(&hits);
    again_hits.set(1.5);
    println!("{} {}", hits.get(), Rc::strong_count(&hits));

    let total = Rc::new(Cell::new(1u32));
    let alias = Rc::clone(&total);
    alias.set(2);
    println!("{}", total.get());

    let (a, b) = pair("text".to_string());
    println!("{} {}", a.value, Rc::strong_count(&b));
    let (c, d) = pair(3);
    drop(c);
    println!("{} {}", d.value, Rc::strong_count(&d));

    let flag = Rc::new(Pair {
        first: true,
        second: 1,
    });
    let flag_again = again(&flag);
    println!("{} {}", flag_again.first, Rc::strong_count(&flag));
    let measure = Rc::new(Pair {
        first: 2.5,
        second: 2,
    });
    let measure_again = again(&measure);
    println!("{} {}", measure_again.first, measure_again.second + measure.second);
}
