// `into_iter()` of an iterator of the crate's is the iterator itself, as
// chrono's `StrftimeItems::parse` asks its own.
struct Count(u32);

impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 3 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
}

impl Count {
    fn doubled(self) -> Vec<u32> {
        self.into_iter().map(|n| n * 2).collect()
    }
}

fn main() {
    println!("{:?}", Count(0).doubled());
    let mut it = Count(1).into_iter();
    println!("{:?} {:?}", it.next(), it.collect::<Vec<_>>());
}
