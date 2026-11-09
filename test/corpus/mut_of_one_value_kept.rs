//@ library-differs
// A `&mut` of a type of one value, `()` or a unit struct, given where the
// call may keep it, for a type parameter what it gives back holds: no
// write changes the value, so it's given as a box of it, kept or not, as
// rustc's async-await/issues/issue-55809's `bad(&mut v).await` (#1061).
trait Touch {
    fn touch(&mut self) -> &'static str;
}

impl Touch for () {
    fn touch(&mut self) -> &'static str {
        "unit"
    }
}

#[derive(Debug, PartialEq)]
struct Marker;

impl Touch for Marker {
    fn touch(&mut self) -> &'static str {
        *self = Marker;
        "marker"
    }
}

impl<T: Touch> Touch for &mut T {
    fn touch(&mut self) -> &'static str {
        (**self).touch()
    }
}

fn kept<T: Touch>(mut t: T) -> T {
    println!("touched {}", t.touch());
    t
}

fn main() {
    let mut unit = ();
    let mut back = kept(&mut unit);
    println!("again {}", back.touch());
    let mut marker = Marker;
    let held = kept(&mut marker);
    *held = Marker;
    println!("{:?} {:?}", held, kept(&mut ()));
    println!("{}", marker == Marker);
}
