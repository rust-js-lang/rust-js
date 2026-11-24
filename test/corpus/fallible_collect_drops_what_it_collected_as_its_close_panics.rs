// A `collect()` into an `Option` or a `Result` that stops at a `None` or
// an `Err` closes what it steps, as std's `from_iter` drops the
// `GenericShunt` it's given: where that drop panics, what it collected is
// dropped as the panic unwinds, then the `Err` it kept, as `try_process`'s
// frame holds it (#2564). Each is run by `catch_unwind`, which takes the
// panic; the last stops at no `None`, so nothing's held as it closes.
use std::panic;

#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop D{}", self.0);
    }
}

struct Held<T>(Vec<T>);

impl<T> Iterator for Held<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }
}

impl<T> Drop for Held<T> {
    fn drop(&mut self) {
        println!("drop Held, {} left", self.0.len());
        panic!("Held's drop");
    }
}

fn options<I: Iterator<Item = Option<D>>>(items: I) -> Option<Vec<D>> {
    items.collect()
}

fn results<I: Iterator<Item = Result<D, D>>>(items: I) -> Result<Vec<D>, D> {
    items.collect()
}

fn main() {
    let stopped = panic::catch_unwind(|| options(Held(vec![Some(D(1)), Some(D(2)), None, Some(D(3))])));
    println!("{}", stopped.is_err());
    let errs = panic::catch_unwind(|| results(Held(vec![Ok(D(4)), Ok(D(5)), Err(D(9)), Ok(D(6))])));
    println!("{}", errs.is_err());
    let direct = panic::catch_unwind(|| Held(vec![Ok(D(7)), Err(D(10)), Ok(D(8))]).collect::<Result<Vec<D>, D>>());
    println!("{}", direct.is_err());
    let mapped = panic::catch_unwind(|| {
        Held(vec![11, 0, 12])
            .map(|x| if x == 0 { None } else { Some(D(x)) })
            .collect::<Option<Vec<D>>>()
    });
    println!("{}", mapped.is_err());
    let whole = panic::catch_unwind(|| options(Held(vec![Some(D(13))])));
    println!("{}", whole.is_err());
}
