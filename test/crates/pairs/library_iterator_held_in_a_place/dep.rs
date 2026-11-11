//! A library's `from_iter` that moves what generic code's `collect()` gives
//! it into a value that holds it, an `Rc<RefCell<..>>`, a box, an `Option`,
//! a field, an array or a tuple: its consumer compiles it from the
//! library's optimized MIR (ADR 0914), and the iterator is dropped with
//! what holds it, as the crate's own is, not as it's stepped to its end
//! (#1366, #1270).
use std::cell::RefCell;
use std::rc::Rc;

pub struct Loud(pub &'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub struct Count {
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

pub struct Holder<I> {
    it: I,
}

pub fn count(name: &'static str) -> Count {
    Count {
        n: 0,
        _loud: Loud(name),
    }
}

pub struct Summed<const PLACE: u8>(pub u32);

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

pub fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}
