//! A consumer giving the library's generic functions values with
//! destructors, which their `Rc`s drop as the last of them goes (#1332).
struct Loud(u32);

#[derive(Clone, Copy, Debug)]
struct Point {
    n: u32,
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    println!("{:?}", dep::counted(Loud(1)));
    println!("{}", dep::shared(Loud(2)));
    println!("{:?}", dep::weakly(Loud(3)));
    let mut p = Point { n: 1 };
    p.n += 1;
    let (n, q) = dep::counted_copy(p);
    p.n += 10;
    println!("{n} {q:?} {p:?}");
}
