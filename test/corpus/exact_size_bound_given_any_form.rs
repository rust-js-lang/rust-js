// What generic code bound by `ExactSizeIterator` is given answers its
// `len`, and its back where it's double-ended, whatever form a crate-wide
// fact of its type made it (#2193, #2194): a `take(n)` made `$taking` where
// another function's `Vec` grows by its hint, of std's compiled `Fuse` or
// `Peekable`, knows its length by its hint, as std's `Take` does, and its
// back once its source is made knowing it; an `inspect` made JS's own map,
// where another function boxes a `rev()` of its zip, knows its source's
// length, as std's `Inspect` does; and a zip's length is its hint's, of its
// sides' `size_hint`s, never their `len`s; a `step_by`'s, its hint's.
struct Count(u32);

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 5 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (5 - self.0) as usize;
        println!("own size_hint {left}");
        (left, Some(left))
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<u32> {
        if self.0 == 5 {
            return None;
        }
        self.0 += 1;
        Some(6 - self.0)
    }
}

impl ExactSizeIterator for Count {
    fn len(&self) -> usize {
        println!("own len");
        (5 - self.0) as usize
    }
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn pairs_hint(it: impl DoubleEndedIterator<Item = (u32, u32)> + ExactSizeIterator) {
    println!("pairs hint {:?}", it.size_hint());
}

fn collected<J: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(j: J) {
    let o: Option<Vec<_>> = Box::new(j).map(Some).collect();
    if let Some(v) = o {
        println!("collected {v:?}, room {}", v.capacity() >= v.len());
    }
}

fn hint_of(it: impl DoubleEndedIterator<Item = u32> + ExactSizeIterator) {
    println!("hint {:?}", it.size_hint());
}

fn len_of(it: impl ExactSizeIterator<Item = u32>) {
    println!("len {}", it.len());
}

fn rfolded<J: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(j: J) {
    let held = Loud("f");
    let n = j.rfold(0, move |a, x| {
        let _h = &held;
        a * 10 + x
    });
    println!("rfold {n}");
}

fn main() {
    let boxed = Box::new(Box::new(Count(0).zip(10..13u32).rev()));
    println!("boxed hint {:?}", boxed.size_hint());
    pairs_hint(Count(0).zip(10..13u32).inspect(|x| println!("saw {}", x.0)));
    println!("zip len {}", Count(1).zip(10..13u32).len());
    collected((1..6u32).fuse().take(3));
    collected((1..6u32).peekable().take(3));
    hint_of((1..6u32).fuse().take(3));
    hint_of((1..6u32).peekable().take(3));
    hint_of((1..10u32).fuse().step_by(3));
    len_of((1..6u32).fuse().take(3));
    rfolded((1..6u32).fuse().skip(1));
    rfolded((1..6u32).fuse().take(3));
}
