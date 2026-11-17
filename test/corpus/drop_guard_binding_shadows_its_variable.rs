// A guard's `Drop` declares its own `previous`, beside where it reads the
// field made of the function's `previous`: inlined, the field's reads
// can't be the function's variable, which the `Drop`'s would hide there,
// so the field is a `const` of its own, spelled apart, `previous$1`, and
// each reads what it did, a one-field tuple struct's too (ADR 1150).
use std::cell::Cell;

thread_local! {
    static RESTORED: Cell<u32> = const { Cell::new(0) };
}

struct Doubled {
    previous: u32,
}

impl Drop for Doubled {
    fn drop(&mut self) {
        if self.previous > 0 {
            let previous = self.previous * 2;
            RESTORED.set(previous + self.previous);
            println!("restored {previous} of {}", self.previous);
        }
    }
}

struct Tripled(u32);

impl Drop for Tripled {
    fn drop(&mut self) {
        if self.0 > 0 {
            let previous = self.0 * 3;
            RESTORED.set(previous + self.0);
            println!("tripled {previous} of {}", self.0);
        }
    }
}

fn tripled(previous: u32) -> u32 {
    let _tripled = Tripled(previous);
    previous + 30
}

fn doubled(previous: u32) -> u32 {
    let _doubled = Doubled { previous };
    previous + 40
}

fn main() {
    println!("{}", doubled(5));
    println!("left {}", RESTORED.get());
    println!("{}", doubled(0));
    println!("left {}", RESTORED.get());
    println!("{}", tripled(4));
    println!("left {}", RESTORED.get());
}
