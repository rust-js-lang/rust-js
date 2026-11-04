// `last()` of a `flatten()` or a `flat_map` is std's `FlattenCompat::last`:
// each inner iterator's own `last`, which drops what it passes, then the
// one kept before dropped, `iter.last().or(last)`, not `Iterator`'s
// default, which drops each item as the next comes (#645): of inner
// iterators that own their items, and of the crate's own that give them;
// one whose inner iterator panics drops the one kept before, then what's
// left of its source; and its source is fused, as std's is.
use std::panic::catch_unwind;
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
/// Gives `D(at)` up to `end`, panicking at `boom`.
struct Upto {
    at: u32,
    end: u32,
    boom: u32,
}
impl Iterator for Upto {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        if self.at == self.boom {
            panic!("boom");
        }
        Some(D(self.at))
    }
}
/// Gives a `Vec` every other step, `None` between, so it isn't fused.
struct Every(u32);
impl Iterator for Every {
    type Item = Vec<D>;
    fn next(&mut self) -> Option<Vec<D>> {
        self.0 += 1;
        match self.0 % 2 {
            1 => Some(vec![D(self.0 + 70)]),
            _ => None,
        }
    }
}
fn main() {
    let last = vec![D(1), D(2)].into_iter().map(|x| vec![D(x.0 + 10), x]).flatten().last();
    println!("last {}", last.as_ref().map_or(0, |d| d.0));
    drop(last);
    let last = (0..3).flat_map(|i| vec![D(i + 20), D(i + 30)]).last();
    println!("last {}", last.as_ref().map_or(0, |d| d.0));
    drop(last);
    let nested = vec![vec![D(40), D(41)], vec![], vec![D(42)], vec![]];
    let last = nested.into_iter().flatten().last();
    println!("last {}", last.as_ref().map_or(0, |d| d.0));
    drop(last);
    let mut it = vec![vec![D(50), D(51)], vec![D(52), D(53)]].into_iter().flatten();
    let first = it.next();
    println!("first {}", first.as_ref().map_or(0, |d| d.0));
    drop(first);
    let last = it.last();
    println!("last {}", last.as_ref().map_or(0, |d| d.0));
    drop(last);
    let last = (0..2)
        .flat_map(|i| Upto { at: i * 10, end: i * 10 + 2, boom: 0 })
        .last();
    println!("last {}", last.as_ref().map_or(0, |d| d.0));
    drop(last);
    let r = catch_unwind(|| {
        let v = vec![D(60), D(61)];
        v.into_iter()
            .flat_map(|d| Upto { at: d.0, end: d.0 + 2, boom: if d.0 == 61 { 62 } else { 0 } })
            .last()
            .map(|d| d.0)
    });
    println!("panicked {}", r.is_err());
    let last = vec![Some(D(80)), None, Some(D(81)), Some(D(82))].into_iter().flatten().last();
    println!("last {}", last.as_ref().map_or(0, |d| d.0));
    drop(last);
    let mut it = Every(0).flatten();
    for d in it.by_ref() {
        println!("got {}", d.0);
    }
    println!("after {}", it.next().map_or(0, |d| d.0));
    println!("last {}", it.last().map_or(0, |d| d.0));
    println!("end");
}
