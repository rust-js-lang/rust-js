// A `map()` of what's stepped from its back elsewhere, made knowing its
// back (`$mapEnds`), passes its source's `size_hint` on, as std's `Map`
// does (map.rs:111): a collect into a `String` asks the crate's own once
// before it steps, as `Extend<char>` reserves by it (#733).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Ends {
    at: u32,
    end: u32,
}

impl Iterator for Ends {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.at < self.end).then(|| {
            self.at += 1;
            D(self.at)
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (self.end - self.at) as usize;
        println!("own hint {left}");
        (left, Some(left))
    }
}

impl DoubleEndedIterator for Ends {
    fn next_back(&mut self) -> Option<D> {
        (self.at < self.end).then(|| {
            self.end -= 1;
            D(self.end + 1)
        })
    }
}

fn ends() -> Ends {
    Ends { at: 0, end: 3 }
}

fn main() {
    let mut it = ends().chain(vec![D(6)]);
    let back = it.next_back();
    println!("back {:?}", back.as_ref().map(|d| d.0));
    let s: String = ends()
        .chain(vec![D(6)])
        .map(|d| char::from(b'a' + d.0 as u8))
        .collect();
    println!("{s}");
}
