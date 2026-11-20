// A `skip(n)` of a `filter()`, `skip(0)` too, is std's `Skip`, whose
// `count` asks its source's `nth(n - 1)` and then its source's `count`:
// `Filter::count`, which asks its source's hint first, here the crate's
// `size_hint` (#2212). Counted here or by generic code.

/// Counts up to `.1` from `.0`, with its own `size_hint`.
struct Own(u32, u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        println!("own next");
        if self.0 == self.1 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        println!("own size_hint {n}");
        (n, Some(n))
    }
}

fn counted<I: Iterator>(it: I) -> usize {
    it.count()
}

fn even(x: &u32) -> bool {
    x % 2 == 0
}

fn main() {
    let chained = vec![7].into_iter().chain(Own(0, 4));
    println!("skipped one: {}", chained.filter(even).skip(1).count());

    let chained = vec![7].into_iter().chain(Own(0, 4));
    println!("skipped none: {}", chained.filter(even).skip(0).count());

    let chained = vec![7].into_iter().chain(Own(0, 4));
    println!("in generic code: {}", counted(chained.filter(even).skip(1)));

    let zipped = vec![7, 8].into_iter().zip(Own(0, 4));
    println!("zipped: {}", counted(zipped.filter(|(x, _)| even(x)).skip(1)));
}
