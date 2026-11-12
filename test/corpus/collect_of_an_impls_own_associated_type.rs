// A generic `collect` of a trait's associated type, `<R as Call<T>>::Out`,
// into a `Vec` std may make in place, asks its items' layout, a built
// type's its caller gives: `MapOk<F>`'s `Out` is `Result<U, E>`, built of
// the impl's own parameters, itertools' `MapSpecialCaseFnOk`, which its
// dictionary can't carry of them (#1348). A collect through a
// `C: FromIterator` bound asks none (#1443), so `into_vec`'s, into a `Vec`
// itself, asks it; where a panic stops it, in place, it drops what it
// collected before what's left (#1430).
trait Call<T> {
    type Out;
    fn call(&mut self, t: T) -> Self::Out;
}

struct MapOk<F>(F);

impl<F, T, U, E> Call<Result<T, E>> for MapOk<F>
where
    F: FnMut(T) -> U,
{
    type Out = Result<U, E>;
    fn call(&mut self, t: Result<T, E>) -> Result<U, E> {
        t.map(&mut self.0)
    }
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
    fn collect<C: FromIterator<Self::Item>>(self) -> C {
        let mut f = self.f;
        self.iter.map(move |v| f.call(v)).collect()
    }
}

impl<I: Iterator, R: Call<I::Item>> Mapped<I, R> {
    fn into_vec(self) -> Vec<R::Out> {
        let mut f = self.f;
        self.iter.map(move |v| f.call(v)).collect()
    }
}

struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let v: Vec<Result<u32, String>> = Mapped {
        iter: vec![Ok(1u32), Err("no".to_string()), Ok(3)].into_iter(),
        f: MapOk(|x: u32| x * 10),
    }
    .collect();
    println!("{v:?}");
    let v: Vec<Result<Loud, u8>> = Mapped {
        iter: vec![Ok(1u32), Err(2u8), Ok(3)].into_iter(),
        f: MapOk(Loud),
    }
    .collect();
    println!("{}", v.len());
    let v = Mapped {
        iter: vec![Ok(1u32), Err(2u8), Ok(3)].into_iter(),
        f: MapOk(Loud),
    }
    .into_vec();
    println!("{}", v.len());
    let r = std::panic::catch_unwind(|| {
        Mapped {
            iter: vec![Ok(Loud(1)), Err(2u8), Ok(Loud(3)), Ok(Loud(4))].into_iter(),
            f: MapOk(|l: Loud| if l.0 == 3 { panic!("at 3") } else { Loud(l.0 * 10) }),
        }
        .into_vec()
        .len()
    });
    println!("{:?}", r.ok());
}
