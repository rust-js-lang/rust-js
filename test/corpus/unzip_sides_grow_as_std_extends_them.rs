// `unzip()` into `Vec`s, each side reserved the lower bound of what it
// steps, as std's `default_extend` asks its hint once, then grown as each
// item's pushed: a `Vec`'s `into_iter()`, whose bound is its length, left
// with room for exactly that; a `filter`'s, whose bound is 0, grown 4, 8,
// 16; a `skip(n)`'s, whose bound is what it has left, and a chain's, the
// sum of its parts'; a `TrustedLen`'s, how many it gives, whatever its
// JS form hints, a `map` by a named function's, and of two `u8`s, std's
// least, 8; and a `BTreeMap`'s `iter()`, exact-sized and not `TrustedLen`,
// skipped and chained, whose JS forms are made knowing their lengths
// (#2405).
use std::collections::BTreeMap;

struct Counted(u32);

impl Iterator for Counted {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<(u32, u32)> {
        self.0 += 1;
        (self.0 <= 5).then_some((self.0, self.0 * 2))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint asked at {}", self.0);
        (5 - self.0 as usize, Some(5))
    }
}

fn shown(name: &str, (a, b): (Vec<u32>, Vec<u32>)) {
    println!("{name}: {} {} | {} {}", a.len(), a.capacity(), b.len(), b.capacity());
}

fn swapped((a, b): (u32, u32)) -> (u32, u32) {
    (b, a)
}

fn swapped_ref(&(a, b): &(u32, u32)) -> (u32, u32) {
    (b, a)
}

fn main() {
    let pairs: Vec<(u32, u32)> = (0..10).map(|n| (n, n * 2)).collect();
    shown("all", pairs.clone().into_iter().unzip());
    shown("kept", pairs.clone().into_iter().filter(|&(n, _)| n % 3 != 0).unzip());
    shown("skipped", pairs.iter().skip(3).copied().unzip());
    shown(
        "chained",
        pairs
            .iter()
            .skip(1)
            .chain(pairs.iter().filter(|&&(n, _)| n < 3))
            .copied()
            .unzip(),
    );
    let (few, bytes): (Vec<u32>, Vec<u8>) = pairs.iter().take(2).map(|&(a, b)| (a, b as u8)).unzip();
    println!(
        "few: {} {} | {} {}",
        few.len(),
        few.capacity(),
        bytes.len(),
        bytes.capacity()
    );
    shown("named", pairs.clone().into_iter().map(swapped).unzip());
    shown("skipped named", pairs.iter().skip(3).map(swapped_ref).unzip());
    shown(
        "chained named",
        pairs
            .iter()
            .skip(1)
            .map(swapped_ref)
            .chain(pairs.iter().filter(|&&(n, _)| n < 3).copied())
            .unzip(),
    );
    let tree: BTreeMap<u32, u32> = pairs.iter().copied().collect();
    shown("tree skipped", tree.iter().skip(2).map(|(&k, &v)| (k, v)).unzip());
    shown(
        "tree chained",
        tree.iter()
            .skip(1)
            .map(|(&k, &v)| (k, v))
            .chain(pairs.iter().filter(|p| p.0 < 2).copied())
            .unzip(),
    );
    shown("mapped", pairs.into_iter().map(|(a, b)| (b, a)).unzip());
    shown("own", Counted(0).unzip());
}
