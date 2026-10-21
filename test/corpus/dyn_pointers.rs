// A raw pointer to a `dyn Trait` is the `dyn`'s pair, as a `&dyn Trait` and
// a `Box<dyn Trait>` are (ADR 0562): made from a thin pointer, called and
// written through, made thin again, and a boxed one taken apart and put
// back, as anyhow does. From rustc's dst-raw.rs.

trait Shape {
    fn area(&self) -> u32;
    fn grow(&mut self, by: u32);
    fn name(&self) -> &'static str {
        "shape"
    }
}

struct Square {
    side: u32,
}

impl Shape for Square {
    fn area(&self) -> u32 {
        self.side * self.side
    }
    fn grow(&mut self, by: u32) {
        self.side += by;
    }
    fn name(&self) -> &'static str {
        "square"
    }
}

struct Rect {
    w: u32,
    h: u32,
}

impl Shape for Rect {
    fn area(&self) -> u32 {
        self.w * self.h
    }
    fn grow(&mut self, by: u32) {
        self.w += by;
    }
}

// A holder of a raw `dyn` pointer, as a registry keeps what it doesn't own.
struct Slot {
    shape: *const dyn Shape,
}

impl Slot {
    fn area(&self) -> u32 {
        unsafe { (*self.shape).area() }
    }
}

fn main() {
    let mut square = Square { side: 3 };
    let z: *const dyn Shape = &square;
    let area = unsafe { (&*z).area() };
    let named = unsafe { (*z).name() };
    println!("{area} {named} {}", z.is_null());

    // Written through a `*mut`.
    let m: *mut dyn Shape = &mut square;
    unsafe { (*m).grow(2) };
    println!("{} {}", square.side, unsafe { (*m).area() });

    // Thin again: the pointer to the value.
    let thin = z as *const Square;
    println!("{} {}", unsafe { (*thin).side }, thin == &raw const square);

    // A boxed `dyn` taken apart and put back.
    let boxed: Box<dyn Shape> = Box::new(Rect { w: 4, h: 4 });
    let raw = Box::into_raw(boxed);
    unsafe { (*raw).grow(1) };
    let back = unsafe { Box::from_raw(raw) };
    println!("{} {}", back.area(), back.name());

    // Compared: the same value and vtable, `==`; the same value whatever the
    // vtable, `addr_eq`.
    let again: *const dyn Shape = &square;
    let twin = Square { side: 5 };
    let other: *const dyn Shape = &twin;
    let as_named: *const dyn Named = &square;
    println!(
        "{} {} {} {}",
        z == again,
        z == other,
        std::ptr::addr_eq(z, again),
        std::ptr::addr_eq(z, as_named)
    );

    // Kept in a struct, called later.
    let rect = Rect { w: 2, h: 5 };
    let slots = [Slot { shape: &square }, Slot { shape: &rect }];
    let areas: Vec<u32> = slots.iter().map(Slot::area).collect();
    println!("{areas:?}");
}

// Another trait the square is, another vtable.
trait Named {
    fn label(&self) -> &'static str;
}

impl Named for Square {
    fn label(&self) -> &'static str {
        "square"
    }
}
