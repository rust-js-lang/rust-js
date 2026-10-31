// A loop that drops a value as it's left, each jump in it where it goes:
// a value assigned in it drops the one it replaces each time, through an
// early `break`, an inner `continue` or a `continue` of the outer loop
// (#440, after drop_reassigned_in_inner_loop's nested loops); and a `for`
// over a borrow, one `try`, whose inner loop's `continue 'outer` still
// names the outer loop, where a `continue` of none never ended.

use std::cell::RefCell;

fn id<T>(x: T) -> T {
    x
}

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn one_loop() {
    let mut held = Noisy(id(30));
    for i in 0..id(3) {
        held = Noisy(id(31 + i));
    }
    println!("one loop ends with {}", held.0);
}

fn nested_whiles() {
    let mut held = Noisy(id(40));
    let mut i = 0;
    while i < id(2) {
        let mut j = 0;
        while j < id(2) {
            held = Noisy(id(41 + i * 2 + j));
            j += 1;
        }
        i += 1;
    }
    println!("whiles end with {}", held.0);
}

fn early_break() {
    let mut held = Noisy(id(50));
    for i in 0..id(5) {
        for j in 0..id(3) {
            held = Noisy(id(51 + i * 3 + j));
            if j == 1 {
                break;
            }
        }
        if i == 1 {
            held = Noisy(id(59));
            break;
        }
    }
    println!("early break ends with {}", held.0);
}

fn inner_continue() {
    let mut held = Noisy(id(60));
    for i in 0..id(2) {
        for j in 0..id(3) {
            if j == 1 {
                continue;
            }
            held = Noisy(id(61 + i * 3 + j));
        }
    }
    println!("inner continue ends with {}", held.0);
}

fn outer_continue() {
    let mut held = Noisy(id(70));
    'outer: for i in 0..id(3) {
        for j in 0..id(3) {
            if j == i {
                continue 'outer;
            }
            held = Noisy(id(71 + i * 3 + j));
        }
    }
    println!("outer continue ends with {}", held.0);
}

// A loop over a borrow with a loop in it that goes on with the outer one,
// `continue 'outer`: one `try` of the outer loop, whose `continue` in the
// inner loop still names it, as a `continue` of none would go on with the
// inner one, never ending.
fn borrowed(items: &RefCell<Vec<u32>>, seen: &dyn Fn(u32)) -> u32 {
    let mut total = 0;
    'outer: for &x in items.borrow().iter() {
        let mut j = 0;
        while j < 3 {
            if j == x {
                continue 'outer;
            }
            total += j;
            j += 1;
        }
        seen(x);
    }
    total
}

fn main() {
    one_loop();
    nested_whiles();
    early_break();
    inner_continue();
    outer_continue();
    let items = RefCell::new(vec![1, 2, 5, 7]);
    println!("{}", borrowed(&items, &|x| println!("seen {x}")));
    items.borrow_mut().push(9);
    println!("{:?}", items.borrow());
}
