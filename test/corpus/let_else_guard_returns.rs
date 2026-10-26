// A `let … else { return a; }` before a `return` the program wrote, whose
// two returns MIR joins after its test, is a guard first, as written (ADR
// 0879, amended): in an `if` whose function's tail is the guard's value
// too, at a function's top, two in a row, and in a loop.

use std::cell::RefCell;

thread_local! {
    static CURRENT: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn current_stack(dev: bool) -> String {
    if dev {
        let Some(current) = CURRENT.with_borrow(|current| current.clone()) else {
            return String::new();
        };
        // The stack of what's current.
        return current + "!";
    }
    String::new()
}

fn named(name: Option<&str>) -> String {
    let Some(name) = name else {
        return String::from("-");
    };
    return name.to_uppercase();
}

fn both(a: Option<u32>, b: Option<u32>) -> u32 {
    let Some(a) = a else {
        return 0;
    };
    let Some(b) = b else {
        return a;
    };
    return a * 10 + b;
}

fn first_even(items: &[Option<u32>]) -> u32 {
    for item in items {
        let Some(n) = item else {
            return 99;
        };
        if n % 2 == 0 {
            return *n;
        }
    }
    0
}

fn main() {
    println!("{:?}", current_stack(true));
    CURRENT.set(Some("app".to_string()));
    println!("{:?} {:?}", current_stack(true), current_stack(false));
    println!("{} {}", named(Some("ab")), named(None));
    println!("{} {} {}", both(None, Some(1)), both(Some(2), None), both(Some(2), Some(3)));
    println!(
        "{} {} {}",
        first_even(&[Some(1), Some(4)]),
        first_even(&[Some(1), None, Some(2)]),
        first_even(&[Some(3)])
    );
}
