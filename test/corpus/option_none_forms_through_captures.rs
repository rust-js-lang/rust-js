// What a test of an `Option` asks is MIR's answer of the one `None` it can
// be (#2112 C S5): a local's writes', followed into a closure that
// captures it, by value or by reference, and through a borrow or a copy
// of the capture, the receiver of `is_none()`; a field borrowed through a
// captured reference, as its trust says; `as_ref()` of a local, which is
// the local in JS; and an `Option` of a value with a destructor, dropped
// by its variants, one of them moved out of in part. Each tested where it's `None` and where it isn't.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Holder {
    #[cfg_attr(rust_js, rust_js::nullable)]
    label: Option<String>,
}

fn moved(flag: bool) -> Box<dyn Fn() -> bool> {
    let name = if flag { Some("named") } else { None };
    Box::new(move || name.is_none())
}

fn borrowed(flag: bool) -> bool {
    let name = if flag { Some("named") } else { None };
    let check = || name.is_some();
    check()
}

fn through_a_field(holder: &Holder) -> usize {
    let read = || match &holder.label {
        Some(label) => label.len(),
        None => 0,
    };
    read()
}

fn lent(flag: bool) -> Option<usize> {
    let name = if flag { Some("named") } else { None };
    name.as_ref().map(|n| n.len())
}

fn dropped(flag: bool) -> u32 {
    let pair = if flag { Some((Loud(7), Loud(8))) } else { None };
    let n = match pair {
        Some((first, _)) => first.0,
        None => 0,
    };
    println!("after");
    n
}

fn main() {
    println!("{} {}", moved(true)(), moved(false)());
    println!("{} {}", borrowed(true), borrowed(false));
    let some = Holder {
        label: Some("abc".to_string()),
    };
    let none = Holder { label: None };
    println!("{} {}", through_a_field(&some), through_a_field(&none));
    println!("{:?} {:?}", lent(true), lent(false));
    println!("{} {}", dropped(true), dropped(false));
}
