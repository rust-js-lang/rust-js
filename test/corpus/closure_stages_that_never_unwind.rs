// A closure is one of the crate's functions that never unwind where its
// MIR can't panic, as a function is (ADR 0902): a collect of items with a
// destructor whose closure stages can't panic, building, adding (which
// wraps, as release Rust's does), printing a number or calling a function
// that never unwinds, needs no closing for a panic; one whose closure
// indexes past the end, divides by zero or asserts still drops what it
// collected as the panic unwinds; one of a generic function counts as one
// that may, as the function does (#701).
use std::panic;

struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn shown(n: u8) -> u8 {
    println!("shown {n}");
    n
}

fn pick(table: &[u8], at: u8) -> u8 {
    table[at as usize]
}

fn tagged<T: Copy + Into<u8>>(items: &[T]) -> Vec<Loud> {
    items.iter().map(|&item| Loud(item.into())).collect()
}

struct Holder<T>(T);

impl<T> Drop for Holder<T> {
    fn drop(&mut self) {
        println!("drop holder");
    }
}

fn held<T: Copy>(items: &[T]) -> Vec<Holder<T>> {
    items.iter().map(|&item| Holder(item)).collect()
}

fn main() {
    let v = vec![1u8, 2, 3];
    let offset = 250;
    let louder: Vec<Loud> = v.iter().map(|&n| Loud(n + offset)).collect();
    println!("louder {}", louder.len());
    let printed: Vec<Loud> = (v.iter())
        .filter(|&&n| n != 2)
        .map(|&n| {
            println!("make {n}");
            Loud(shown(n))
        })
        .collect();
    println!("printed {}", printed.len());
    println!("tagged {}", tagged(&v).len());
    println!("held {}", held(&v).len());

    let table = [5u8, 6];
    let r = panic::catch_unwind(|| v.iter().map(|&n| Loud(table[n as usize])).collect::<Vec<_>>());
    println!("indexed {}", r.is_err());
    let r = panic::catch_unwind(|| v.iter().map(|&n| Loud(6 / (2 - n))).collect::<Vec<_>>());
    println!("divided {}", r.is_err());
    let r = panic::catch_unwind(|| {
        (v.iter())
            .map(|&n| {
                assert!(n < 3, "too big {n}");
                Loud(n)
            })
            .collect::<Vec<_>>()
    });
    println!("asserted {}", r.is_err());
    let r = panic::catch_unwind(|| v.iter().map(|&n| Loud(pick(&table, n))).collect::<Vec<_>>());
    println!("picked {}", r.is_err());
}
