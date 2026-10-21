// A pointer rustc's const-eval made (ADR 0562): a null, an integer made a
// pointer, as semver's `HEAD` is, a dangling one, and one to memory rustc
// interned, `&42 as *const i32`, in constants and in statics' fields, each
// made as its initializer says. From rustc's const-cast-ptr-int.rs and
// static-raw-pointer-interning.rs.
use std::ptr::{self, NonNull};

struct Slot {
    x: *const u8,
}

unsafe impl Sync for Slot {}

impl Slot {
    // semver's `Prerelease::EMPTY`: an associated constant holding one.
    const EMPTY: Slot = Slot { x: ptr::null() };
}

struct Interned {
    field: *const i32,
    tag: u8,
}

unsafe impl Sync for Interned {}

static NULLED: Slot = Slot { x: 0 as *const u8 };
static INTERNED: Interned = Interned {
    field: &42 as *const i32,
    tag: 7,
};

// semver's empty identifier: all ones, a pointer no allocation has.
const HEAD: NonNull<u8> = unsafe { NonNull::new_unchecked(!0 as *mut u8) };
const DANGLING: *const u64 = ptr::dangling();

const NOWHERE: *const u8 = ptr::null();

// semver's `empty()`: the constant in the function's own body.
fn head() -> NonNull<u8> {
    const LOCAL: NonNull<u8> = unsafe { NonNull::new_unchecked(!0 as *mut u8) };
    LOCAL
}

fn main() {
    println!(
        "{} {} {} {}",
        NULLED.x.is_null(),
        unsafe { *INTERNED.field },
        INTERNED.tag,
        NOWHERE.is_null()
    );
    println!(
        "{} {} {}",
        HEAD.as_ptr() as usize == usize::MAX,
        HEAD.as_ptr().is_null(),
        DANGLING as usize
    );
    println!("{} {} {}", head() == HEAD, head().as_ptr().is_null(), Slot::EMPTY.x.is_null());
}
