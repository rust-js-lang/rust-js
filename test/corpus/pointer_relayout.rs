// A pointer cast between pointees keeps the place where rustc's layout puts
// the target there (ADR 0562): a `#[repr(C)]` struct's pointer as its first
// field's, and a struct erased to its prefix and unerased again, as anyhow
// keeps an error's vtable first and reaches it through `*const ErrorImpl<()>`.
use std::fmt;
use std::ptr::NonNull;

struct VTable {
    name: &'static str,
    show: unsafe fn(NonNull<Erased>) -> String,
}

#[repr(C)]
struct Impl<E = ()> {
    vtable: &'static VTable,
    count: u32,
    object: E,
}

type Erased = Impl<()>;

// The vtable read through the struct's pointer cast to its first field's.
fn vtable_of(p: NonNull<Erased>) -> &'static VTable {
    unsafe { *(p.as_ptr() as *const &'static VTable) }
}

// The erased pointer as the pointer to the whole it was made from.
unsafe fn show_of<E: fmt::Display>(p: NonNull<Erased>) -> String {
    let unerased = p.cast::<Impl<E>>();
    let inner = unsafe { unerased.as_ref() };
    format!("{} #{}", inner.object, inner.count)
}

static NUMBER: VTable = VTable {
    name: "number",
    show: show_of::<u32>,
};

static TEXT: VTable = VTable {
    name: "text",
    show: show_of::<&'static str>,
};

fn erased<E>(inner: Box<Impl<E>>) -> NonNull<Erased> {
    NonNull::from(Box::leak(inner)).cast::<Erased>()
}

fn main() {
    let boxed = Box::new(Impl {
        vtable: &NUMBER,
        count: 1,
        object: 7u32,
    });
    let number = erased(boxed);
    let text = erased(Box::new(Impl {
        vtable: &TEXT,
        count: 2,
        object: "seven",
    }));
    for p in [number, text] {
        let vtable = vtable_of(p);
        println!("{} {}", vtable.name, unsafe { (vtable.show)(p) });
    }

    // The erased struct's own fields read where it is, and unerased ones.
    let count = unsafe { p_count(number) };
    let whole = unsafe { Box::from_raw(number.cast::<Impl<u32>>().as_ptr()) };
    println!("{count} {} {}", whole.object, whole.vtable.name);
    drop(unsafe { Box::from_raw(text.cast::<Impl<&'static str>>().as_ptr()) });
}

unsafe fn p_count(p: NonNull<Erased>) -> u32 {
    unsafe { p.as_ref() }.count
}
