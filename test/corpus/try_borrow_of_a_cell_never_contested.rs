// `try_borrow()` and `try_borrow_mut()` of a `RefCell` no borrow of which
// can meet another (ADR 0863): each `Ok`, as Rust's is, the cell itself,
// unchecked, its value read and written through the guard, the `Err` arm
// never taken (#1376: decision coverage found no program reaching it but
// the metamorphic net's).
use std::cell::RefCell;

struct Tally {
    count: RefCell<u32>,
    names: RefCell<Vec<String>>,
}

impl Tally {
    fn bump(&self) -> u32 {
        match self.count.try_borrow_mut() {
            Ok(mut n) => {
                *n += 1;
                *n
            }
            Err(_) => 0,
        }
    }

    fn read(&self) -> Option<u32> {
        self.count.try_borrow().ok().map(|n| *n)
    }

    fn name(&self, name: &str) -> usize {
        if let Ok(mut names) = self.names.try_borrow_mut() {
            names.push(name.to_string());
        }
        self.names.try_borrow().map(|names| names.len()).unwrap_or(usize::MAX)
    }
}

pub fn main() {
    let tally = Tally {
        count: RefCell::new(0),
        names: RefCell::new(Vec::new()),
    };
    println!("{} {}", tally.bump(), tally.bump());
    println!("{:?}", tally.read());
    println!("{} {}", tally.name("a"), tally.name("b"));
    let cell = RefCell::new(String::from("x"));
    let joined = match cell.try_borrow() {
        Ok(s) => format!("{s}!"),
        Err(e) => format!("{e}"),
    };
    println!("{joined} {}", cell.try_borrow_mut().is_ok());
}
