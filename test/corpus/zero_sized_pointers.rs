// A pointer to a value of no size reads and writes through any address that
// isn't null, as Rust's does (ADR 0562): a unit struct behind a dangling
// pointer, `&*p` of one, an empty byte string's pointer, and an empty
// slice made of a dangling pointer, whose pointer is that address, equal to
// another empty slice's. From rustc's deref-chain-method-calls-13264.rs,
// const-err-rpass.rs and issue-105536-const-val-roundtrip-ptr-eq.rs.
use std::ptr::NonNull;
use std::slice::from_raw_parts;

struct Node;

impl Node {
    fn name(&self) -> &'static str {
        "node"
    }
}

#[derive(Clone, Copy)]
struct Handle {
    node: *const Node,
}

impl Handle {
    fn get(&self) -> &Node {
        unsafe { &*self.node }
    }
}

const EMPTY: *const u8 = b"" as *const _ as *const u8;
const DANGLING: *const u8 = NonNull::dangling().as_ptr();
const EMPTY_REF: &[u8] = unsafe { from_raw_parts(DANGLING, 0) };

fn main() {
    let handle = Handle {
        node: NonNull::dangling().as_ptr(),
    };
    println!("{} {}", handle.get().name(), handle.node.is_null());
    unsafe { handle.node.cast_mut().write(Node) };

    let unit: *mut () = NonNull::dangling().as_ptr();
    unsafe {
        unit.write(());
        let () = unit.read();
    }
    println!("{}", EMPTY.is_null());

    let made = unsafe { from_raw_parts(DANGLING, 0) };
    println!(
        "{} {} {} {}",
        made.is_empty(),
        made.as_ptr() == DANGLING,
        made.as_ptr() == EMPTY_REF.as_ptr(),
        EMPTY_REF.as_ptr() == DANGLING
    );

    // An empty Vec allocates nothing: its pointer is its items' dangling
    // address, as every empty Vec of them is.
    let none: Vec<u16> = Vec::new();
    let other: Vec<u16> = Vec::with_capacity(0);
    println!(
        "{} {}",
        none.as_ptr() == NonNull::<u16>::dangling().as_ptr(),
        none.as_ptr() == other.as_ptr()
    );
}
