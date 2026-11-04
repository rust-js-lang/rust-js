// A function's scope struct whose value its JS reads whole, a counted
// `RefCell`'s field cell, `$fieldCell(tally, "n")`, or whose fields
// are a tuple's, keeps its object, its fields its properties, where its
// methods are still functions declared in the function (ADR 1043, amended).
use std::cell::RefCell;

struct Tally {
    n: RefCell<u32>,
    pair: (u32, u32),
}

impl Tally {
    fn bump(&self) -> u32 {
        let mut n = self.n.borrow_mut();
        *n += self.pair.0;
        *n
    }

    fn bump_if(&self, go: bool) -> u32 {
        if go { self.bump() } else { self.pair.1 }
    }
}

struct Pair(u32, String);

impl Pair {
    fn show(&self) -> String {
        format!("{}{}", self.0, self.1)
    }
}

fn run(k: u32) -> String {
    let tally = Tally {
        n: RefCell::new(k),
        pair: (2, 3),
    };
    tally.bump();
    let pair = Pair(k, "x".to_string());
    format!(
        "{} {} {} {}",
        tally.bump(),
        tally.n.borrow(),
        tally.bump_if(false),
        pair.show()
    )
}

fn main() {
    println!("{} {}", run(1), run(2));
}
