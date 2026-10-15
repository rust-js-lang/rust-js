// A `&mut` of what a generic `Option`'s `Some` holds, bound by a pattern of
// a `&mut Option<T>`: a handle that reads what's in the `Some` and writes the
// option a `Some` of what it's given, `$someMut(o)` (ADR 0051), whether it's
// read, written, or read in a guard, as chrono's `set_if_consistent` does.
fn write<T>(o: &mut Option<T>, v: T) {
    if let Some(x) = o {
        *x = v;
    }
}

fn read<T: PartialEq>(o: &mut Option<T>, v: &T) -> bool {
    match o {
        Some(x) => *x == *v,
        None => false,
    }
}

fn set_if_consistent<T: PartialEq>(old: &mut Option<T>, new: T) -> Result<(), ()> {
    match old {
        Some(old) if *old != new => Err(()),
        _ => {
            *old = Some(new);
            Ok(())
        }
    }
}

fn main() {
    let mut a = Some(1);
    write(&mut a, 2);
    // A `Some` that looks like `None`, boxed, written a `Some` that doesn't.
    let mut n: Option<Option<i32>> = Some(None);
    write(&mut n, Some(5));
    let mut back: Option<Option<i32>> = Some(Some(1));
    write(&mut back, None);
    let r = read(&mut a, &2);
    println!("{:?} {:?} {:?} {}", a, n, back, r);
    let mut year: Option<i64> = None;
    println!("{:?}", set_if_consistent(&mut year, 2024));
    println!("{:?}", set_if_consistent(&mut year, 2024));
    println!("{:?}", set_if_consistent(&mut year, 1999));
    println!("{:?}", year);
    let mut name: Option<String> = None;
    println!("{:?}", set_if_consistent(&mut name, "Ada".to_string()));
    println!("{:?} {:?}", set_if_consistent(&mut name, "Bo".to_string()), name);
}
