// An iterator moved into a value that holds it, an `Rc<RefCell<..>>`, a
// box, an `Option`, a field, an array or a tuple, is dropped with that
// value, where Rust drops it, not as it's stepped to its end (#1270): here
// what generic code's `collect()` gives the crate's own `from_iter`, which
// generic code closes where it's done unless what holds it is held.
use std::cell::RefCell;
use std::rc::Rc;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Count {
    n: u32,
    _loud: Loud,
}

impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.n < 2).then(|| {
            self.n += 1;
            self.n
        })
    }
}

struct Holder<I> {
    it: I,
}

fn count(name: &'static str) -> Count {
    Count { n: 0, _loud: Loud(name) }
}

struct Summed<const PLACE: u8>(u32);

impl<const PLACE: u8> FromIterator<u32> for Summed<PLACE> {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let mut sum = 0;
        match PLACE {
            0 => {
                let it = Rc::new(RefCell::new(items.into_iter()));
                while let Some(n) = it.borrow_mut().next() {
                    sum += n;
                }
                println!("cell summed {sum}");
            }
            1 => {
                let mut it = Box::new(items.into_iter());
                while let Some(n) = (*it).next() {
                    sum += n;
                }
                println!("box summed {sum}");
            }
            2 => {
                let mut it = Some(items.into_iter());
                while let Some(n) = it.as_mut().unwrap().next() {
                    sum += n;
                }
                println!("option summed {sum}");
            }
            3 => {
                let mut held = Holder { it: items.into_iter() };
                while let Some(n) = held.it.next() {
                    sum += n;
                }
                println!("field summed {sum}");
            }
            4 => {
                let mut held = [items.into_iter()];
                while let Some(n) = held[0].next() {
                    sum += n;
                }
                println!("array summed {sum}");
            }
            _ => {
                let mut held = (items.into_iter(), 0);
                while let Some(n) = held.0.next() {
                    sum += n;
                }
                println!("tuple summed {sum}");
            }
        }
        Summed(sum)
    }
}

fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn main() {
    let _: Summed<0> = collected(count("cell").filter(|_| true));
    let _: Summed<1> = collected(count("box").filter(|_| true));
    let _: Summed<2> = collected(count("option").filter(|_| true));
    let _: Summed<3> = collected(count("field").filter(|_| true));
    let _: Summed<4> = collected(count("array").filter(|_| true));
    let _: Summed<5> = collected(count("tuple").filter(|_| true));
}
