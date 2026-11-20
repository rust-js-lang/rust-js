// What's made knowing its back is asked of the value, by the MIR local
// that holds it, never of its type (#2158): a held `map` of `Ends`, and a
// variable's, through a `&mut` too, are std's compiled structs beside
// another `Ends` stepped from its back. A variable stepped from its back
// itself, or lent `&mut` and stepped so, is made knowing it, each answer
// native's.
struct Ends(u32, u32);

impl Iterator for Ends {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 < self.1 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
}

impl DoubleEndedIterator for Ends {
    fn next_back(&mut self) -> Option<u32> {
        if self.0 < self.1 {
            self.1 -= 1;
            Some(self.1 + 1)
        } else {
            None
        }
    }
}

fn double(x: u32) -> u32 {
    x * 2
}

fn main() {
    // Another value of its type, stepped from its back.
    let mut other = Ends(2, 9);
    println!("{:?}", other.next_back());
    let held = Ends(1, 4).map(double as fn(u32) -> u32);
    println!("{:?}", held.enumerate().last());
    let apart = Ends(1, 5);
    println!("{:?}", apart.map(double).enumerate().last());
    // Lent `&mut` and stepped from its front alone.
    let mut fresh = Ends(4, 7);
    println!("{:?}", (&mut fresh).map(double).enumerate().last());
    println!("{:?}", fresh.next());
    println!("{:?}", other.next());

    // The value itself, stepped from its back before it's chained.
    let mut stepped = Ends(0, 6);
    println!("{:?}", stepped.next_back());
    println!("{:?}", stepped.map(double).enumerate().last());

    // Lent `&mut`, stepped from its back through the borrow, then chained.
    let mut lent = Ends(3, 8);
    {
        let borrowed = &mut lent;
        println!("{:?}", borrowed.next_back());
    }
    println!("{:?}", (&mut lent).map(double).enumerate().last());
    println!("{:?}", lent.next());
}
