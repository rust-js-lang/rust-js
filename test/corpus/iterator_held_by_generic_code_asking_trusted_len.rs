//@ library-differs
// The crate's iterator holding a type parameter, whose `next` grows a
// `Vec` by `extend(it.by_ref().take(1))`, which asks whether that
// parameter is `TrustedLen`, made a JS iterator by generic code that holds
// it: given where a generic iterator goes, and folded through a `&mut` to
// it by std's `fold`. Each caller says what its parameter is, a range,
// which is, or the crate's own iterator, which isn't (#2327).
struct Pool<I: Iterator> {
    it: I,
    items: Vec<I::Item>,
}

impl<I: Iterator> Iterator for Pool<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.items.extend(self.it.by_ref().take(1));
        self.items.pop()
    }
}

struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

fn total<J: Iterator<Item = u32>>(items: J) -> u32 {
    items.sum()
}

fn pooled<I: Iterator<Item = u32>>(pool: Pool<I>) -> u32 {
    total(pool)
}

fn folded<I: Iterator<Item = u32>>(mut pool: Pool<I>) -> (u32, usize) {
    let digits = pool.by_ref().fold(0, |sum, n| sum * 10 + n);
    (digits, pool.items.len())
}

fn main() {
    println!("{}", pooled(Pool { it: 0..4, items: Vec::new() }));
    println!("{}", pooled(Pool { it: Countdown(5), items: Vec::new() }));
    println!("{:?}", folded(Pool { it: 1..5, items: Vec::new() }));
    println!("{:?}", folded(Pool { it: Countdown(4), items: Vec::new() }));
}
