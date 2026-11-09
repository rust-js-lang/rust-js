//@ native: wasm32
// A variant's field is inside its enum, where rustc's layout of that variant
// puts it, past the tag: matched, `if let Shape::Rect { corner, .. } =
// &shape`, its address and its fields' are the enum's and that offset,
// whichever is asked first, of a local, an array's item or a struct's
// field. A variant's field was its object's own allocation, `4100` where
// wasm32 gives `8` (#1114).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct P {
    a: u8,
    b: u32,
}

#[derive(Debug)]
enum Shape {
    Dot(u8, P),
    Rect { width: u16, corner: P },
}

struct Holder {
    n: u8,
    shape: Shape,
}

fn corner_address(shape: &Shape) -> usize {
    match shape {
        Shape::Rect { corner, .. } => &raw const corner.b as usize,
        Shape::Dot(_, p) => p as *const P as usize,
    }
}

fn main() {
    let rect = Shape::Rect { width: 3, corner: P { a: 1, b: 2 } };
    let base = &raw const rect as usize;
    if let Shape::Rect { corner, width } = &rect {
        println!("named {} {} {width}", &raw const corner.b as usize - base, corner as *const P as usize - base);
    }
    let dot = Shape::Dot(4, P { a: 5, b: 6 });
    let field_first = corner_address(&dot);
    println!("tuple {}", field_first - &raw const dot as usize);

    let shapes = [Shape::Dot(7, P { a: 8, b: 9 }), Shape::Rect { width: 10, corner: P { a: 11, b: 12 } }];
    let start = &raw const shapes as usize;
    let size = std::mem::size_of::<Shape>();
    println!("items {} {}", corner_address(&shapes[0]) - start, corner_address(&shapes[1]) - start - size);

    let holder = Holder { n: 13, shape: Shape::Rect { width: 14, corner: P { a: 15, b: 16 } } };
    let at = corner_address(&holder.shape);
    println!("field {}", at - &raw const holder as usize);
    println!("{:?} {:?} {:?} {}", rect, dot, shapes, holder.n);
}
