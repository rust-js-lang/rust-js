// A flatten of arrays collected where nothing reads a capacity, its source
// the crate's own iterator, whose `size_hint` prints: std's collect asks
// the flatten's hint once, exact by its items' constant size, so it asks
// the source's once; held there, std's struct, which knows it (#1327).
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
}
