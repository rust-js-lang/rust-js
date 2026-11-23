// `unzip()` asks what it steps its `size_hint()` once, before its first
// item, as std's `default_extend` does to reserve each side, though
// nothing reads a capacity: so a crate's own hint runs, here and through
// a `filter` and a `flat_map`, whose hints ask their source's (#2405).
struct Counted(u32);

impl Iterator for Counted {
    type Item = (u32, char);
    fn next(&mut self) -> Option<(u32, char)> {
        self.0 += 1;
        println!("next {}", self.0);
        (self.0 <= 3).then_some((self.0, (b'a' + self.0 as u8) as char))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint asked at {}", self.0);
        (3 - self.0 as usize, Some(3))
    }
}

fn main() {
    let (numbers, letters): (Vec<u32>, Vec<char>) = Counted(0).unzip();
    println!("{numbers:?} {letters:?}");
    let (odd, odd_letters): (Vec<u32>, Vec<char>) = Counted(0).filter(|&(n, _)| n % 2 == 1).unzip();
    println!("{odd:?} {odd_letters:?}");
    let (swapped, chained): (Vec<char>, Vec<u32>) = Counted(0)
        .map(|(n, c)| (c, n))
        .chain(Counted(2).map(|(n, c)| (c, n)))
        .unzip();
    println!("{swapped:?} {chained:?}");
    // A flatten's hint asks what it flattens, so the crate's runs.
    let (flat, flat_letters): (Vec<u32>, Vec<char>) = Counted(1).flat_map(|pair| [pair, pair]).unzip();
    println!("{flat:?} {flat_letters:?}");
}
