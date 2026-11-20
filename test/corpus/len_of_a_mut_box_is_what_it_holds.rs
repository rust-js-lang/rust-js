// `len()` and `next_back()` of a `&mut` of a `Box` are what the box holds',
// as std's `&mut I` and `Box<I>` each pass them on: of std's `map` over the
// crate's iterator, its hint, the crate's own `size_hint`; of the crate's
// iterator boxed, its own `size_hint`, never the default `(0, None)`; of a
// slice's items, how many are left (#2189).
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
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("own size_hint {}", self.0);
        (self.0 as usize, Some(self.0 as usize))
    }
}

impl ExactSizeIterator for Countdown {}

fn main() {
    let mut mapped = Box::new(Countdown(3).map(|x| x * 2));
    let r = &mut mapped;
    println!("mapped {}", r.len());
    r.next();
    println!("mapped {}", r.len());

    let mut own = Box::new(Countdown(4));
    let r = &mut own;
    println!("own {}", r.len());

    let mut items = Box::new([1, 2, 3].iter().map(|x| x * 10));
    let r = &mut items;
    println!("items {} {:?} {}", r.len(), r.next_back(), r.len());
}
