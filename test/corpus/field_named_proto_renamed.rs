// A field named `__proto__` in Rust given another JS name, `proto` or
// `link`, is an object's own field as any other, however it's made, set,
// copied or replaced: a struct's, a variant's, a union's and a class's
// (rd754). A `HashMap`'s `"__proto__"` key is a `Map`'s, and a tuple's
// fields are indexes.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Default)]
struct Optional {
    #[cfg_attr(rust_js, rust_js::name = "proto")]
    __proto__: Option<u32>,
    others: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
enum Shape {
    Linked {
        #[cfg_attr(rust_js, rust_js::name = "proto")]
        __proto__: Option<u32>,
        size: u32,
    },
    Tuple(u32, Option<u32>),
}

union Bits {
    #[cfg_attr(rust_js, rust_js::name = "proto")]
    __proto__: u32,
    other: i32,
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Chained {
    #[cfg_attr(rust_js, rust_js::name = "link")]
    __proto__: Option<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Chained {
    pub fn new(proto: Option<u32>) -> Self {
        Chained { __proto__: proto }
    }

    pub fn proto(&self) -> Option<u32> {
        self.__proto__
    }

    pub fn set(&mut self, proto: Option<u32>) {
        self.__proto__ = proto;
    }
}

fn made(proto: Option<u32>) -> Optional {
    Optional {
        __proto__: proto,
        others: vec![1],
    }
}

fn reset(optional: &mut Optional) {
    *optional = Optional::default();
}

fn become_linked(shape: &mut Shape) {
    *shape = Shape::Linked {
        __proto__: Some(8),
        size: 2,
    };
}

fn main() {
    let mut none = made(None);
    println!("{none:?} {}", none.__proto__.is_none());
    none.__proto__ = Some(4);
    let copy = none.clone();
    none.__proto__ = None;
    println!("{none:?} {copy:?} {}", none == copy);
    println!("{}", Optional::default() == made(None));
    let updated = Optional {
        __proto__: Some(3),
        ..copy.clone()
    };
    println!("{updated:?}");

    let mut held = made(Some(6));
    let old = std::mem::replace(&mut held, made(None));
    println!("{old:?} {held:?}");
    let mut other = made(Some(7));
    std::mem::swap(&mut held, &mut other);
    println!("{held:?} {other:?}");
    reset(&mut held);
    println!("{held:?} {}", held.__proto__.is_none());

    let mut shape = Shape::Tuple(3, None);
    become_linked(&mut shape);
    if let Shape::Linked { __proto__, .. } = &mut shape {
        *__proto__ = None;
    }
    println!("{shape:?} {}", shape == Shape::Linked { __proto__: None, size: 2 });

    let mut bits = Bits { other: -1 };
    bits.__proto__ = 3;
    println!("{}", unsafe { bits.__proto__ });
    bits.other = 4;
    println!("{}", unsafe { bits.__proto__ });

    let mut chained = Chained::new(None);
    println!("{:?}", chained.proto());
    chained.set(Some(2));
    println!("{:?}", chained.proto());

    let mut map = HashMap::new();
    map.insert("__proto__".to_string(), 1);
    println!("{:?} {}", map.get("__proto__"), map.len());
}
