//! A crate given a library's `RefCell`s, which it represents and borrows as
//! the library does (ADR 0863): a plain field's read and replaced in place,
//! and the `Rc`'s array and the `Set` borrowed for a moment, unchecked.
pub fn main() {
    let outer = dep::start(None);
    dep::add_type(&outer, "a");
    let inner = dep::start(Some(&outer));
    dep::add_type(&inner, "b");
    dep::add_type(&inner, "a");
    println!("{:?} {}", dep::types(&outer), dep::share_types(&outer, &inner));
    if let Some(types) = inner.types.borrow().as_ref() {
        types.borrow_mut().push("c".to_string());
    }
    let replaced = inner.types.replace(None);
    println!(
        "{:?} {:?} {}",
        dep::types(&outer),
        dep::types(&inner),
        replaced.is_some()
    );
    dep::visit(&inner, 1);
    dep::visit(&inner, 1);
    dep::visit(&inner, 2);
    if let Some(seen) = &inner.seen {
        println!("{}", seen.borrow().len());
    }
    println!("{} {}", dep::finish(&inner), dep::finish(&inner));
    let made = dep::Transition {
        types: std::cell::RefCell::new(None),
        seen: None,
    };
    dep::add_type(&made, "made");
    println!("{:?} {}", dep::types(&made), dep::finish(&made));
}
