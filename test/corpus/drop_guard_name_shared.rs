// A one-field value's `Drop`, written where it's dropped, takes apart the
// guard it's given only where that name is the guard's alone: a closure
// given a `Counted` reads its field, though another closure's guard has
// its name (#239).
struct Counted(u32);

impl Drop for Counted {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn make(n: u32) -> Counted {
    Counted(n)
}

fn main() {
    let total: u32 = vec![make(7), make(8)].into_iter().map(|counted| counted.0).sum();
    println!("{total}");
    let fresh = |n: u32| Counted(n + 1).0;
    println!("{}", fresh(1013));
}
