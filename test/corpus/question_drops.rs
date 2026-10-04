// `?` of a value with a destructor: what it unwraps passes straight to where
// `e?` goes, and what it returns early with straight out, each dropped once,
// where Rust drops it. As num-traits' `checked_pow` does, in generic code.

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn step(d: &D) -> Option<D> {
    if d.0 < 3 { Some(D(d.0 + 1)) } else { None }
}

fn climb(mut d: D) -> Option<D> {
    d = step(&d)?;
    d = step(&d)?;
    Some(d)
}

fn first(d: D) -> Option<u32> {
    let e = step(&d)?;
    Some(e.0 + d.0)
}

fn discarded(d: &D) -> Option<u32> {
    step(d)?;
    Some(0)
}

fn fallible(d: &D) -> Result<D, D> {
    if d.0 % 2 == 0 { Ok(D(d.0 + 100)) } else { Err(D(d.0 + 200)) }
}

fn go(d: D) -> Result<u32, D> {
    let e = fallible(&d)?;
    Ok(e.0)
}

trait CheckedDouble: Sized {
    fn checked_double(&self) -> Option<Self>;
}

impl CheckedDouble for D {
    fn checked_double(&self) -> Option<D> {
        if self.0 < 10 { Some(D(self.0 * 2)) } else { None }
    }
}

fn twice<T: CheckedDouble>(mut base: T) -> Option<T> {
    base = base.checked_double()?;
    base = base.checked_double()?;
    Some(base)
}

fn main() {
    for start in [0, 2] {
        match climb(D(start)) {
            Some(d) => println!("climbed to {}", d.0),
            None => println!("stopped"),
        }
    }
    println!("{:?}", first(D(1)));
    println!("{:?}", first(D(5)));
    println!("{:?}", discarded(&D(0)));
    for start in [2, 3] {
        match go(D(start)) {
            Ok(n) => println!("ok {}", n),
            Err(e) => println!("err {}", e.0),
        }
    }
    for start in [1, 3] {
        match twice(D(start)) {
            Some(d) => println!("doubled to {}", d.0),
            None => println!("too big"),
        }
    }
}
