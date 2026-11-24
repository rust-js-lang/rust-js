//@ library-differs
// A `match` whose arms that meet go on to a body they share, and another
// that goes on past it, inside an `if`: one test of the arms that meet,
// whose branch holds what they share, and the other its `else`, as a person
// writes it, no `break` out of a block around the `if` (#2144). Its tests
// read the tag again, so only where reading it runs no code: of an enum JS
// may give, whose tag a getter might answer, the tag read once into a
// `const` first (#2208), which they read.
use std::ops::Bound;

pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

enum Mapped<T> {
    Single(T),
    Ambiguous(T, T),
    None,
}

fn taken_if(mapped: Mapped<Loud>, take: bool) {
    if take {
        match mapped {
            Mapped::Single(t) | Mapped::Ambiguous(t, _) => println!("took {}", t.0),
            Mapped::None => println!("none"),
        }
    }
    println!("left");
}

/// What JS may give, std's `Bound`, not the crate's own data: its tag a
/// getter might answer, read once, `const kind = bound.TAG`, which the
/// tests read.
pub fn bound_if(bound: Bound<Loud>, take: bool) {
    if take {
        match bound {
            Bound::Included(t) | Bound::Excluded(t) => println!("bound {}", t.0),
            Bound::Unbounded => println!("unbounded"),
        }
    }
    println!("done");
}

fn main() {
    taken_if(Mapped::Single(Loud(1)), true);
    taken_if(Mapped::Ambiguous(Loud(2), Loud(3)), true);
    taken_if(Mapped::None, true);
    taken_if(Mapped::Single(Loud(4)), false);
    bound_if(Bound::Included(Loud(6)), true);
    bound_if(Bound::Excluded(Loud(7)), true);
    bound_if(Bound::Unbounded, true);
    bound_if(Bound::Included(Loud(9)), false);
}
