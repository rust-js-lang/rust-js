// An `and_then` of a `#[rust_js::nullable]` function, whose `None` is
// `null`, stored in a nullable place as it is (ADR 0841, amended); of one
// that may give `undefined`, coalesced. Each read back after.

use std::cell::RefCell;

struct Fiber {
    id: u32,
}

struct Info {
    #[cfg_attr(rust_js, rust_js::nullable)]
    source: Option<Box<Fiber>>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static NAME: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn name_of(fiber: &Fiber) -> Option<String> {
    if fiber.id > 2 { Some(format!("#{}", fiber.id)) } else { None }
}

fn maybe(fiber: &Fiber) -> Option<String> {
    if fiber.id > 2 { Some(fiber.id.to_string()) } else { None }
}

fn logged(info: &Info) -> Option<String> {
    NAME.set(info.source.as_deref().and_then(name_of));
    NAME.with_borrow(Clone::clone)
}

fn maybe_logged(info: &Info) -> Option<String> {
    NAME.set(info.source.as_deref().and_then(maybe));
    NAME.with_borrow(Clone::clone)
}

fn main() {
    let infos = [
        Info { source: Some(Box::new(Fiber { id: 3 })) },
        Info { source: Some(Box::new(Fiber { id: 1 })) },
        Info { source: None },
    ];
    for info in &infos {
        println!("{:?} {:?}", logged(info), maybe_logged(info));
        println!("{}", NAME.with_borrow(|name| name.is_none()));
    }
}
