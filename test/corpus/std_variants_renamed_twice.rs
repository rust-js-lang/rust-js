// Two `use`s that rename std's `Option` and its variants apart, as
// itertools' `multi_product.rs` does: std's are no import of the file's,
// so naming them twice is no second name of one import (ADR 0202, amended,
// #473).

use Option::{self as State, None as ProductEnded, Some as ProductInProgress};
use Option::{self as CurrentItems, None as NotYetPopulated, Some as Populated};

fn step(state: State<u32>) -> State<u32> {
    match state {
        ProductInProgress(n) if n < 3 => ProductInProgress(n + 1),
        ProductInProgress(_) | ProductEnded => ProductEnded,
    }
}

fn first(items: &CurrentItems<Vec<u32>>) -> CurrentItems<u32> {
    match items {
        Populated(items) => items.first().copied(),
        NotYetPopulated => NotYetPopulated,
    }
}

fn main() {
    let mut state = ProductInProgress(1);
    while let ProductInProgress(n) = state {
        println!("in progress {n}");
        state = step(state);
    }
    println!("ended {}", state.is_none());
    println!("{:?} {:?}", first(&Populated(vec![7, 8])), first(&NotYetPopulated));
}
