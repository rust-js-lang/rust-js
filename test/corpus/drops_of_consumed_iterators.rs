// What's dropped where an iterator is consumed: an `impl Iterator` of the
// crate's own, as the iterator it hides is, and of a `collect` into a
// `Result` or an `Option` that stops, what's left of its source, then each
// value it collected, as std's are.
struct Counter(u32);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 2 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        println!("drop counter at {}", self.0);
    }
}

fn count() -> impl Iterator<Item = u32> {
    Counter(0)
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let s: u32 = count().sum();
    println!("s {s}");
    let doubled: Vec<u32> = count().map(|x| x * 2).collect();
    println!("{doubled:?}");

    let r: Result<Vec<D>, u32> = vec![Ok(D(1)), Err(7), Ok(D(2))].into_iter().collect();
    println!("err {}", r.is_err());
    let o: Option<Vec<D>> = vec![Some(D(3)), None, Some(D(4))].into_iter().collect();
    println!("none {}", o.is_none());
    let all: Result<Vec<D>, u32> = vec![Ok(D(5))].into_iter().collect();
    println!("ok {}", all.is_ok());
}
