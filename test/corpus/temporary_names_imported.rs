// A temporary isn't named after a function another module gives this one,
// whose import keeps its name (ADR 0569): `find` and `prepare` stay the
// calls the source writes, and the temporaries they make take the next
// name, the type they hold or `value`.
mod util {
    pub fn find(points: &[i32], at: i32) -> Option<i32> {
        points.iter().copied().find(|&p| p == at)
    }

    #[derive(Debug)]
    pub enum Prepared {
        Ready(u32),
        Blocked,
    }

    pub fn prepare(n: u32) -> Prepared {
        if n > 1 { Prepared::Ready(n) } else { Prepared::Blocked }
    }
}

use util::{Prepared, find, prepare};

fn main() {
    let points = [1, 2, 3];
    if let Some(p) = find(&points, 2) {
        println!("{p} {:?}", find(&points, 3));
    }
    println!("{:?}", find(&points, 1));
    match prepare(3) {
        Prepared::Ready(n) => println!("ready {n} {:?}", prepare(0)),
        Prepared::Blocked => println!("blocked"),
    }
}
