//@ native: wasm32
// `cycle()` holds its iterator and a clone of it as it was; each time the
// one it steps ends, it's replaced by a clone of the original, which drops
// the one replaced, and dropped, it drops both, as std's `Cycle` does. In
// the build of std's bodies it is std's compiled `Cycle`, made by std's
// compiled `cycle` (#1458).
#[derive(Debug)]
struct Count {
    tag: u32,
    at: u32,
    end: u32,
}

impl Clone for Count {
    fn clone(&self) -> Count {
        println!("clone {} at {}", self.tag, self.at);
        Count {
            tag: self.tag + 10,
            ..*self
        }
    }
}

impl Drop for Count {
    fn drop(&mut self) {
        println!("drop {} at {}", self.tag, self.at);
    }
}

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(self.at)
    }
}

fn count(tag: u32, end: u32) -> Count {
    Count { tag, at: 0, end }
}

fn main() {
    let taken: Vec<u32> = count(1, 2).cycle().take(5).collect();
    println!("{taken:?}");
    let mut it = count(2, 3).cycle();
    println!("{:?}", it.size_hint());
    println!("{:?}", it.nth(4));
    println!("{:?}", it.next());
    drop(it);
    let mut empty = count(3, 0).cycle();
    println!("{:?}", empty.next());
    drop(empty);
    let found = count(4, 3).cycle().skip(1).position(|x| x == 1);
    println!("{found:?}");
    let over = count(5, 3)
        .cycle()
        .try_fold(0u32, |acc, x| if acc > 10 { None } else { Some(acc + x) });
    println!("{over:?}");
    let found = [1u8, 2, 3].iter().cycle().position(|&x| x == 2);
    println!("{found:?}");
    let sum: u32 = (1..4).cycle().take(7).sum();
    println!("{sum}");
    let mut stepped = (0..3).cycle();
    println!("{:?} {:?}", stepped.nth(7), stepped.size_hint());
    let mut none = (0..0).cycle();
    println!("{:?} {:?}", none.size_hint(), none.next());
    let words: Vec<String> = ["a", "b"].iter().cycle().take(3).map(|s| s.to_string()).collect();
    println!("{words:?}");
    let zipped: Vec<(u32, u8)> = count(6, 1).cycle().zip(0..2).collect();
    println!("{zipped:?}");
}
