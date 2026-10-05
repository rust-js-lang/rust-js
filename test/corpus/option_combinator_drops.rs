// `filter` and `map_or` of an `Option` whose value has a destructor: what
// `filter` doesn't keep is dropped once its test says so, and `map_or`'s
// fallback once the function has run, as chrono's do.

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

enum Found {
    Nothing,
    One(D),
}

fn make(n: u32) -> Option<D> {
    if n > 0 { Some(D(n)) } else { None }
}

fn even(d: &D) -> bool {
    println!("test {}", d.0);
    d.0 % 2 == 0
}

// In generic code, a `T` whose `Some` is boxed (ADR 0051).
fn keep<T>(value: Option<T>, test: impl Fn(&T) -> bool) -> Option<T> {
    value.filter(test)
}

fn found<T>(value: Option<T>, fallback: T) -> T {
    value.map_or(fallback, |t| t)
}

fn main() {
    let kept = make(2).filter(even);
    println!("kept {}", kept.is_some());
    let dropped = make(3).filter(even);
    println!("dropped {}", dropped.is_none());
    println!("none {}", make(0).filter(even).is_none());
    let closure = make(5).filter(|d| d.0 > 4);
    println!("closure {}", closure.as_ref().map_or(0, |d| d.0));
    println!("generic {}", keep(make(6), |d| d.0 > 9).is_none());
    println!("generic {}", keep(Some(make(7)), |d| d.is_some()).is_some());

    let one = make(8).map_or(Found::Nothing, Found::One);
    if let Found::One(d) = &one {
        println!("one {}", d.0);
    }
    let nothing = make(0).map_or(Found::One(D(9)), Found::One);
    println!("nothing {}", matches!(nothing, Found::One(_)));
    let read = make(10).map_or(D(11), |d| D(d.0 + 100));
    println!("read {}", read.0);
    println!("found {}", found(make(12), D(13)).0);
    println!("found {}", found(None, D(14)).0);
    println!("end");
}
