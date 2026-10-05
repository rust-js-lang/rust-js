// A temporary in a match arm's body ends with the arm, before what the arm
// binds, as chrono's `MappedLocalTime::and_then` matches `(f(min), f(max))`.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn make(name: &'static str, give: bool) -> Option<Loud> {
    println!("make {name}");
    if give { Some(Loud(name)) } else { None }
}

enum Mapped<T> {
    None,
    Single(T),
    Ambiguous(T, T),
}

impl<T> Mapped<T> {
    fn and_then<U, F: FnMut(T) -> Option<U>>(self, mut f: F) -> Mapped<U> {
        match self {
            Mapped::None => Mapped::None,
            Mapped::Single(v) => match f(v) {
                Some(new) => Mapped::Single(new),
                None => Mapped::None,
            },
            Mapped::Ambiguous(min, max) => match (f(min), f(max)) {
                (Some(min), Some(max)) => Mapped::Ambiguous(min, max),
                _ => Mapped::None,
            },
        }
    }

    fn count(&self) -> usize {
        match self {
            Mapped::None => 0,
            Mapped::Single(_) => 1,
            Mapped::Ambiguous(..) => 2,
        }
    }
}

fn first(k: Option<&'static str>) -> usize {
    match k {
        Some(name) => {
            let held = Loud("held");
            match make(name, true) {
                Some(l) => {
                    println!("got {} with {}", l.0, held.0);
                    l.0.len()
                }
                None => 0,
            }
        }
        None => Loud("arm").0.len(),
    }
}

fn main() {
    let both = Mapped::Ambiguous("a", "b").and_then(|n| make(n, true));
    println!("both {}", both.count());
    let one = Mapped::Ambiguous("c", "d").and_then(|n| make(n, n == "c"));
    println!("one {}", one.count());
    let single = Mapped::Single("e").and_then(|n| make(n, true));
    println!("single {}", single.count());
    let none = Mapped::Single("f").and_then(|n| make(n, false));
    println!("none {}", none.count());
    println!("first {}", first(Some("g")));
    println!("first {}", first(None));
    println!("end");
}
