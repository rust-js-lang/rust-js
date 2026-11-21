// A `&mut Box<I>` read back through a generic trait, `GetMut<Box<I>>`, whose
// `I` that bound gives as it is: its `enumerate().skip(n)` asks the crate's
// own `nth`, as std's `Skip::next` asks `Enumerate::nth`, which asks its
// source's, which `&mut I` and `Box<I>` pass on, whatever is known of its
// back or its length; as does one of a `Box<dyn Iterator>` of the crate's
// iterator a `skip`, a `step_by` or `nth` asks through, given to generic
// code or held, and where nothing asks `nth` of it, a loop's, it's JS's own
// map (#2238).

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {}
}

struct Every {
    at: u32,
    end: u32,
}

impl Iterator for Every {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(D(self.at - 1))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.at) as usize;
        (n, Some(n))
    }
    fn nth(&mut self, n: usize) -> Option<D> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

impl ExactSizeIterator for Every {}

impl DoubleEndedIterator for Every {
    fn next_back(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.end -= 1;
        Some(D(self.end))
    }
}

trait GetMut<T: ?Sized> {
    fn get_mut(&mut self) -> &mut T;
}

impl<T: ?Sized> GetMut<T> for T {
    fn get_mut(&mut self) -> &mut T {
        self
    }
}

fn by_dyn<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: &mut dyn GetMut<Box<I>>) {
    let src: &mut Box<I> = GetMut::get_mut(src);
    for (i, x) in src.enumerate().skip(1) {
        println!("dyn {i} {}", x.0);
    }
}

fn by_impl<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: &mut impl GetMut<Box<I>>) {
    let src: &mut Box<I> = GetMut::get_mut(src);
    for (i, x) in src.enumerate().skip(1) {
        println!("impl {i} {}", x.0);
    }
}

struct Count(u32);

impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.checked_sub(1)?;
        Some(self.0)
    }
    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("count nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

fn second<I: Iterator>(mut it: I) -> Option<I::Item> {
    it.nth(1)
}

fn boxed(n: u32) -> Box<dyn Iterator<Item = u32>> {
    Box::new(Count(n))
}

fn main() {
    by_dyn(&mut Box::new(Every { at: 0, end: 3 }));
    by_impl(&mut Box::new(Every { at: 0, end: 3 }));
    println!("given {:?}", second(boxed(4).enumerate()));
    let mut held = boxed(4).enumerate();
    println!("held {:?}", held.nth(1));
    println!("skipped {:?}", boxed(4).enumerate().skip(1).next());
    println!("stepped {:?}", boxed(6).enumerate().step_by(2).nth(1));
    for (i, x) in boxed(3).enumerate() {
        println!("looped {i} {x}");
    }
}
