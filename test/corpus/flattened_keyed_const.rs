// A keyed enum's constants (#504): rustc's values of its variants are its
// keys too, no tag, a `None` one's key there, `{}` of the one without
// fields, each told by its key where it's read; and a variant of a
// `None` made in place, its key there too.
pub enum Shape {
    Dot { at: Option<i32> },
    Line { from: i32, to: i32 },
    Empty,
}

pub struct Mark {
    pub id: u32,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub shape: Shape,
}

const NOWHERE: Shape = Shape::Dot { at: None };
const UNIT: Shape = Shape::Line { from: 0, to: 1 };
const EMPTY: Shape = Shape::Empty;

fn describe(mark: &Mark) -> String {
    match &mark.shape {
        Shape::Dot { at } => format!("{} dot {at:?}", mark.id),
        Shape::Line { from, to } => format!("{} line {from}..{to}", mark.id),
        Shape::Empty => format!("{} empty", mark.id),
    }
}

fn kind(shape: &Shape) -> &'static str {
    match shape {
        Shape::Dot { .. } => "dot",
        Shape::Line { .. } => "line",
        Shape::Empty => "empty",
    }
}

fn main() {
    println!("{} {}", kind(&EMPTY), kind(&NOWHERE));
    let marks = [
        Mark {
            id: 1,
            shape: NOWHERE,
        },
        Mark { id: 2, shape: UNIT },
        Mark {
            id: 3,
            shape: EMPTY,
        },
        Mark {
            id: 4,
            shape: Shape::Dot { at: None },
        },
    ];
    for mark in &marks {
        println!("{}", describe(mark));
    }
}
