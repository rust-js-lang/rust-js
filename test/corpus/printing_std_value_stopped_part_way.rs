//@ library-differs
// A std value's `Debug`, a `Vec`'s, an `Option`'s, a tuple's or a map's, in
// a line, of a part whose writer prints and then panics, the panic caught:
// Rust's stdout keeps what std's builders wrote before the part, `x [!L1,
// !L2, !`, as its `Debug` is its builder's, each piece written as it's taken
// (#1561). Was refused (#1530). `{:#?}` too.
use std::collections::BTreeMap;
use std::fmt;
use std::panic::catch_unwind;

struct Loud(u8);

impl fmt::Debug for Loud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("!");
        assert!(self.0 < 3, "too loud");
        write!(f, "L{}", self.0)
    }
}

#[derive(Debug)]
struct Holds {
    loud: Loud,
}

struct Steps(u32);

impl Iterator for Steps {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        print!("<step>");
        assert!(self.0 < 2, "stepped too far");
        self.0 += 1;
        Some(self.0)
    }
}

struct Listed(u32);

impl fmt::Debug for Listed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(Steps(self.0)).finish()
    }
}

fn main() {
    let r = catch_unwind(|| println!("x {:?} y", vec![Loud(1), Loud(2), Loud(3)]));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:#?} y", vec![Loud(1), Loud(3)]));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:?} y", Some(Loud(4))));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:?} y", (Loud(1), Loud(5))));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:?} y", BTreeMap::from([(1, Loud(1)), (2, Loud(6))])));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:?} y", vec![Holds { loud: Loud(1) }, Holds { loud: Loud(7) }]));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:?} y", vec![Listed(0), Listed(1)]));
    println!(" caught {}", r.is_err());
    println!("x {:?} y", vec![Some(Loud(1)), None]);
}
