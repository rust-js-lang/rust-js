//@ library-differs
// A unit struct generic code writes through a `&mut`, `*r = old`, and
// clones: JS holds it as `undefined`, its one value, which no write
// changes, so it's in no mutation set and its clone is itself (#2251). An
// iterator of the crate's that's a unit struct, stepped by an adapter that
// puts back a clone of it, as itertools' `take_while_ref` does; and a unit
// field of a `Copy` struct lent `&mut` to generic code, which writes the
// field's key on its object, so `b` is a copy of `a`.

#[derive(Clone, Debug)]
struct Marker;

#[derive(Clone, Copy, Debug)]
struct Ticks;

impl Iterator for Ticks {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        Some(7)
    }
}

/// Steps `iter` while `keep` holds, putting back what it was before the
/// item that didn't.
struct WhileRef<'a, I, F> {
    iter: &'a mut I,
    keep: F,
}

impl<I: Iterator + Clone, F: FnMut(&I::Item) -> bool> Iterator for WhileRef<'_, I, F> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let old = self.iter.clone();
        let item = self.iter.next()?;
        if (self.keep)(&item) {
            Some(item)
        } else {
            *self.iter = old;
            None
        }
    }
}

fn while_ref<I: Iterator + Clone, F: FnMut(&I::Item) -> bool>(iter: &mut I, keep: F) -> WhileRef<'_, I, F> {
    WhileRef { iter, keep }
}

fn keep<T: Clone>(r: &mut T) -> T {
    let old = r.clone();
    *r = old.clone();
    old
}

#[derive(Clone, Copy, Debug)]
struct Holder {
    tag: Ticks,
    n: u32,
}

fn main() {
    let mut m = Marker;
    let kept = keep(&mut m);
    let again = kept.clone();
    println!("{m:?} {kept:?} {again:?}");

    let mut ticks = Ticks;
    let mut seen = 0;
    let taken: Vec<u8> = while_ref(&mut ticks, |_| {
        seen += 1;
        seen < 4
    })
    .collect();
    println!("{taken:?} {:?}", ticks.next());

    let a = Holder { tag: Ticks, n: 1 };
    let mut b = a;
    keep(&mut b.tag);
    println!("{a:?} {b:?}");
}
