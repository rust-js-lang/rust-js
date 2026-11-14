// A local lent `&mut`, made a raw pointer of, and written through the
// pointer after its owner reads it: what a test of it showed before the
// write is no longer so (#1782). `n - 1` past `n != 0` wraps where the
// pointer set it to 0; Miri finds no undefined behaviour, as a read
// through the owner leaves the pointer it lent able to write. Assigned
// again, it's what that gave, and a test of it tells again.
fn counted(mut n: u32) -> u32 {
    let p: *mut u32 = &mut n;
    if n == 0 {
        return 0;
    }
    unsafe { *p = 0 };
    n - 1
}

fn reassigned(mut n: u32) -> u32 {
    let p: *mut u32 = &mut n;
    unsafe { *p = 3 };
    n = 9;
    n - 1
}

fn retested(mut n: u32, m: u32) -> u32 {
    let p: *mut u32 = &mut n;
    unsafe { *p = 3 };
    n = m.min(100);
    if n == 0 {
        return 0;
    }
    n - 1
}

fn main() {
    println!("{}", counted(5));
    println!("{}", reassigned(1));
    println!("{}", retested(1, 8));
}
