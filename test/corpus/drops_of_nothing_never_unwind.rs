// A function whose only drops drop nothing never unwinds, so a call of it
// beside a value with a destructor needs no `try` (#717): a `String` it's
// given and drops, which JS drops nothing of, and a value with a
// destructor it makes and gives on, moved out before its drop.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ignored(_text: String, n: u32) -> u32 {
    n + 1
}

fn made(n: u32) -> Loud {
    let loud = Loud(n);
    loud
}

fn outer(text: String, held: Loud) -> u32 {
    let n = ignored(text, held.0);
    let other = made(n);
    other.0 + held.0
}

fn main() {
    let held = Loud(1);
    println!("{}", outer(String::from("x"), held));
}
