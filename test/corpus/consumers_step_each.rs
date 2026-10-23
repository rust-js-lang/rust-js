// A consumer's closure runs as each item comes, between one `next` and the
// next, as std's does: `max_by_key`'s key, `min_by`'s compare and
// `partition`'s predicate, and a `map`'s before them (#179).
struct Up {
    n: u32,
}

impl Iterator for Up {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        println!("next");
        if self.n < 3 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }
}

fn up() -> Up {
    Up { n: 0 }
}

fn main() {
    let max = up().max_by_key(|x| {
        println!("key {x}");
        *x % 2
    });
    println!("{max:?}");
    let min = up().min_by(|a, b| {
        println!("compare {a} {b}");
        b.cmp(a)
    });
    println!("{min:?}");
    let (even, odd): (Vec<u32>, Vec<u32>) = up().partition(|x| {
        println!("keep {x}");
        x % 2 == 0
    });
    println!("{even:?} {odd:?}");
    let values = [3, 1, 2];
    let largest = values
        .iter()
        .map(|x| {
            println!("map {x}");
            x * 10
        })
        .max_by_key(|x| {
            println!("key {x}");
            *x
        });
    println!("{largest:?}");
    println!("{:?}", up().max());
}
