// A flatten of arrays, whose hint std counts by its items' constant size,
// collected or extended where the program sees the hint std's collect
// asks: its source is the crate's own iterator, whose `size_hint` prints.
// std asks it once, the flatten's hint exact; a flatten made in place, JS's
// own `flatMap`, knew no constant size, so a `Vec` asked again as it grew
// (#1327). Held where it's seen; `plain`'s, whose hint runs nothing, made in
// place as it was, its capacity std's all the same (ADR 0419).
use std::collections::VecDeque;

struct Count(u32);

impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 > 0).then(|| {
            self.0 -= 1;
            self.0
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint {}", self.0);
        (self.0 as usize, Some(self.0 as usize))
    }
}

fn main() {
    let v: Vec<u32> = Count(4).flat_map(|n| [n, n + 10]).collect();
    println!("{v:?}");
    let mut w = vec![99];
    w.extend(Count(3).flat_map(|n| [n; 2]));
    println!("{w:?}");
    let mut q = VecDeque::new();
    q.extend(Count(2).flat_map(|n| [n; 3]));
    println!("{q:?}");
    let plain: Vec<u32> = (0..3u32).flat_map(|n| [n, n]).collect();
    println!("{plain:?} {}", plain.capacity());
}
