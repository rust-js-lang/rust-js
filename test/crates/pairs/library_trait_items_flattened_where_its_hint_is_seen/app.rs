// A library's trait's associated `IntoIterator`, flattened by the consumer's
// generic code, collected, or mapped and collected, where the crate's own
// `size_hint` prints: held, std's struct, which asks its items' constant
// size of the trait's dictionaries, which carry it whatever asks it, the
// library's manifest says (#1341): of the consumer's own impl, which makes
// them an array, std asks the hint once, where made in place it asked it
// again as the `Vec` grew; of the library's, a `Range`'s, none.
use dep::Source;

struct Own(u32);

impl Iterator for Own {
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

struct Pairs;

impl Source for Pairs {
    type Items = [u32; 2];
    fn items(n: u32) -> [u32; 2] {
        [n, n + 10]
    }
}

fn gathered<S: Source>() -> Vec<u32> {
    Own(4).flat_map(|n| S::items(n)).collect()
}

fn mapped<S: Source>() -> Vec<u32> {
    Own(3).flat_map(|n| S::items(n)).map(|x| x * 10).collect()
}

pub fn main() {
    println!("{:?}", gathered::<dep::Ranges>());
    println!("{:?}", mapped::<dep::Ranges>());
    println!("{:?}", gathered::<Pairs>());
    println!("{:?}", mapped::<Pairs>());
}
