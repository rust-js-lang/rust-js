//! A `Copy` enum with a fieldless variant, each variant with fields an
//! object (ADR 0033), held by a struct the library changes in place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Circle(i32),
    Dot,
}

pub struct Holder {
    pub shape: Shape,
}

pub fn held() -> Holder {
    Holder {
        shape: Shape::Circle(1),
    }
}

pub fn grow(holder: &mut Holder) {
    if let Shape::Circle(r) = &mut holder.shape {
        *r += 10;
    }
}
