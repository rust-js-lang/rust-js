// A `cycle()` of a `Vec`'s owned items, made in place, closed by what
// consumes it, a `take()` at its end or a `break`, drops the clone it keeps,
// then what it steps, though the first's drop panics, as `Cycle`'s drop glue
// goes on (#883, #1979). Of each item, its clone's generation: what's given
// is 0, the cycle's kept clone 1, whose first item's drop panics.
use std::panic::catch_unwind;

struct D {
    n: u32,
    generation: u32,
}

impl Clone for D {
    fn clone(&self) -> D {
        D { n: self.n, generation: self.generation + 1 }
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {} of generation {}", self.n, self.generation);
        if self.generation == 1 && self.n == 1 {
            panic!("the kept clone's drop");
        }
    }
}

fn given() -> Vec<D> {
    vec![D { n: 1, generation: 0 }, D { n: 2, generation: 0 }, D { n: 3, generation: 0 }]
}

fn main() {
    let r = catch_unwind(|| {
        let n = given().into_iter().cycle().take(2).map(|d| d.n).sum::<u32>();
        println!("sum {n}");
    });
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        for d in given().into_iter().cycle() {
            println!("loop {}", d.n);
            if d.n == 2 {
                break;
            }
        }
    });
    println!("{}", r.is_err());
}
