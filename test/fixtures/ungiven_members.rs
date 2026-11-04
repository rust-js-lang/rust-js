// What test/declarations-ungiven.test.ts declares: a namespace's, an
// object's and a class's members whose JS takes a fact or a dictionary.
use std::any::TypeId;
use std::cell::Cell;
use std::fmt::Display;

#[cfg_attr(rust_js, rust_js::namespace)]
pub struct Sizes;

unsafe impl js::Defined for Sizes {}

#[cfg_attr(rust_js, rust_js::namespace)]
impl Sizes {
    pub fn size_of<T>(&self) -> usize {
        size_of::<T>()
    }

    pub fn count(&self, n: u32) -> u32 {
        n
    }
}

#[cfg_attr(rust_js, rust_js::object)]
pub struct Scope {
    pub queries: Cell<u32>,
}

unsafe impl js::Defined for Scope {}

#[cfg_attr(rust_js, rust_js::object)]
impl Scope {
    pub fn named<T>(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    pub fn count(&self) -> u32 {
        self.queries.get()
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Widget {
    pub size: u32,
}

unsafe impl js::Defined for Widget {}

impl js::Classed for Widget {
    type Constructor = js::AnyConstructor;
}

#[cfg_attr(rust_js, rust_js::class)]
impl Widget {
    pub fn new(size: u32) -> Self {
        Widget { size }
    }

    pub fn is_u32<T: 'static>(&self) -> bool {
        TypeId::of::<T>() == TypeId::of::<u32>()
    }

    pub fn shown<T: Display>(value: T) -> String {
        format!("<{value}>")
    }

    pub fn area(&self) -> u32 {
        self.size * self.size
    }
}

pub trait Sided {
    fn sides(&self) -> u32;
}

impl Sided for Widget {
    fn sides(&self) -> u32 {
        4
    }
}

pub fn total(sizes: &Sizes, scope: &Scope, widget: &Widget) -> usize {
    sizes.size_of::<u64>() + scope.named::<u8>().len() + usize::from(widget.is_u32::<u32>()) + Widget::shown(1).len()
}

