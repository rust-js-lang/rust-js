use dep::pair;
use std::iter::FusedIterator;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Src {
    left: u32,
    held: Loud,
}

impl Iterator for Src {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.next_back()
    }
}

impl DoubleEndedIterator for Src {
    fn next_back(&mut self) -> Option<u32> {
        if self.left == 0 {
            println!("{} asked past its end", self.held.0);
            return None;
        }
        self.left -= 1;
        Some(self.left)
    }
}

impl ExactSizeIterator for Src {
    fn len(&self) -> usize {
        self.left as usize
    }
}

impl FusedIterator for Src {}

struct Plain(Src);

impl Iterator for Plain {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0.next()
    }
}

impl DoubleEndedIterator for Plain {
    fn next_back(&mut self) -> Option<u32> {
        self.0.next_back()
    }
}

impl ExactSizeIterator for Plain {
    fn len(&self) -> usize {
        self.0.len()
    }
}

fn pushed(mut acc: Vec<u32>, x: u32) -> Vec<u32> {
    acc.push(x);
    acc
}

pub fn main() {
    let fused = pair(Src {
        left: 4,
        held: Loud("fused"),
    })
    .rfold(Vec::new(), pushed);
    println!("{fused:?}");
    let plain = pair(Plain(Src {
        left: 3,
        held: Loud("plain"),
    }))
    .rfold(Vec::new(), pushed);
    println!("{plain:?}");
}
