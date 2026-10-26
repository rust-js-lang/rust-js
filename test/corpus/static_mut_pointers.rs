// A raw pointer to a `static mut`, `&raw mut COUNT` or `addr_of_mut!`, is a
// handle on its place (ADRs 0096, amended, 0407): where it's given on, its
// module keeps it in its box, `{ value }`, which the handle writes; where
// it's only read or written through where it's taken, it's the variable.
use std::ptr::{self, addr_of, addr_of_mut};

static mut STEPS: u32 = 0;
static mut COUNT: u32 = 0;
static mut TOTAL: u32 = 0;
static mut OTHER: u32 = 0;
static mut NAMES: Vec<&str> = Vec::new();
static mut BEST: (u8, &str) = (0, "none");

fn add(total: *mut u32, n: u32) {
    unsafe { *total += n };
}

fn bump(count: &mut u32) {
    *count += 1;
}

fn names() -> &'static mut Vec<&'static str> {
    unsafe { &mut *addr_of_mut!(NAMES) }
}

fn main() {
    unsafe {
        *addr_of_mut!(STEPS) += 2;
        bump(&mut *addr_of_mut!(STEPS));
        let steps = *addr_of!(STEPS);
        println!("{steps}");

        *addr_of_mut!(COUNT) += 2;
        let count = &raw mut COUNT;
        *count *= 5;
        bump(&mut *addr_of_mut!(COUNT));
        let read = *addr_of!(COUNT);
        println!("{read}");

        add(&raw mut TOTAL, 3);
        add(addr_of_mut!(TOTAL), 4);
        let total = TOTAL;
        println!("{total}");
        let same = ptr::eq(&raw const TOTAL, addr_of!(TOTAL));
        let other = ptr::eq(&raw const TOTAL, &raw const OTHER);
        println!("{same} {other}");
        ptr::write(&raw mut OTHER, ptr::read(&raw const TOTAL) + 1);
        let other = OTHER;
        println!("{other}");

        names().push("ada");
        (*&raw mut NAMES).push("grace");
        let names = (*&raw const NAMES).clone();
        println!("{names:?}");

        let best = &raw mut BEST;
        (*best).0 = 9;
        (*best).1 = "nine";
        let shown = BEST;
        println!("{shown:?}");
    }
}
