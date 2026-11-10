//@ run-fail: capacity overflow
//@ library-differs
// A crate's own `collect`, itertools' `MapSpecialCase`'s, maps what it holds
// and collects it into its caller's `C: FromIterator`: the `Vec` entry its
// caller gives knows its items' layout, where the generic `collect` doesn't,
// and makes how std's `SpecFromIter` collects it of the `IInPlace` and
// `ITrustedLen` std's `from_iter` gives it (#1239): in place, by its fold, an
// endless `TrustedLen` panicking "capacity overflow" before it's stepped
// again.
trait Call<T> {
    type Out;
    fn call(&mut self, t: T) -> Self::Out;
}

struct Mapped<I, R> {
    iter: I,
    f: R,
}

impl<I: Iterator, R: Call<I::Item>> Iterator for Mapped<I, R> {
    type Item = R::Out;
    fn next(&mut self) -> Option<R::Out> {
        self.iter.next().map(|v| self.f.call(v))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
    fn collect<C: FromIterator<Self::Item>>(self) -> C {
        let mut f = self.f;
        self.iter.map(move |v| f.call(v)).collect()
    }
}

struct Double;
impl Call<u32> for Double {
    type Out = u64;
    fn call(&mut self, t: u32) -> u64 {
        println!("call {t}");
        u64::from(t) * 2
    }
}

fn main() {
    let v: Vec<u64> = Mapped {
        iter: vec![1u32, 2].into_iter(),
        f: Double,
    }
    .collect();
    println!("{v:?}");
    let v: Vec<u64> = Mapped {
        iter: 0..3u32,
        f: Double,
    }
    .collect();
    println!("{v:?}");
    let mut e = std::iter::repeat(5u32);
    e.next();
    let v: Vec<u64> = Mapped { iter: e, f: Double }.collect();
    println!("{v:?}");
}
