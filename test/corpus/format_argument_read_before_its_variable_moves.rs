// A format argument is made in its order, before a later argument moves
// the variable it reads: a move hands the same JS object to its new owner,
// which may change it in place, a range `sum` steps, a struct a `mut`
// parameter, a method's `mut self` or a closure writes (#2201).
use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
struct Point {
    x: i32,
}

impl Point {
    fn shifted(mut self) -> i32 {
        self.x += 10;
        self.x
    }
}

impl Drop for Point {
    fn drop(&mut self) {
        println!("drop {}", self.x);
    }
}

// One with no destructor, whose clone is made where it's read.
#[derive(Debug, Clone)]
struct Plain {
    x: i32,
}

impl Plain {
    fn shifted(mut self) -> i32 {
        self.x += 10;
        self.x
    }
}

fn bump_plain(mut p: Plain) -> i32 {
    p.x += 100;
    p.x
}

fn consume(o: Option<Plain>) -> i32 {
    let mut p = o.unwrap();
    p.x += 50;
    p.x
}

fn pair(t: (Plain, i32)) -> i32 {
    let (mut p, n) = t;
    p.x += n;
    p.x
}

fn bump(mut p: Point) -> i32 {
    p.x += 100;
    p.x
}

fn sum_of(r: &RangeInclusive<i32>) -> String {
    let c = r.clone();
    format!("{:?} {}", c.clone(), c.sum::<i32>())
}

fn main() {
    // A range made by `..=`, then one made by a clone.
    let c = 1..=2;
    println!("{:?} {}", c.clone(), c.sum::<i32>());
    println!("{}", sum_of(&(3..=4)));

    // A field copied, then the struct given to a `mut` parameter.
    let p = Point { x: 1 };
    println!("{} {}", { p.x }, bump(p));

    // A struct's clone, then the struct given to a function or a method's
    // `mut self`.
    let q = Point { x: 2 };
    println!("{:?} {}", q.clone(), bump(q));
    let s = Point { x: 3 };
    println!("{:?} {}", s.clone(), s.shifted());

    // Moved into a closure that changes it, and into a block.
    let t = Point { x: 4 };
    println!(
        "{:?} {}",
        t.clone(),
        (move || {
            let mut t = t;
            t.x += 1000;
            t.x
        })()
    );
    let u = Point { x: 5 };
    println!("{:?} {}", u.clone(), {
        let mut u = u;
        u.x += 1000;
        u.x
    });

    // The same of a struct with no destructor.
    let a = Plain { x: 6 };
    println!("{:?} {}", a.clone(), bump_plain(a));
    let b = Plain { x: 7 };
    println!("{:?} {}", b.clone(), b.shifted());
    let d = Plain { x: 8 };
    println!(
        "{:?} {}",
        d.clone(),
        (move || {
            let mut d = d;
            d.x += 1000;
            d.x
        })()
    );

    // Moved into an `Option` or a tuple, which is given to what changes it.
    let e = Plain { x: 9 };
    println!("{} {}", { e.x }, consume(Some(e)));
    let f = Plain { x: 10 };
    println!("{:?} {}", f.clone(), pair((f, 7)));

    // A range moved into a closure that steps it.
    let r = 1..=3;
    println!("{:?} {}", r.clone(), (move || r.sum::<i32>())());
}
