//@ run-fail: index out of bounds: the len is 0 but the index is 0
// A `Vec` lent `&mut` to code that keeps a raw pointer to it, a function
// or a closure, or std's code that gives a pointer to it back: a later
// call may change its length through the pointer, so a test of the length
// holds only until a call (#1966). `len() - 3` past `len() > 2` wraps
// where the call cleared it, and `v[0]` panics.
static mut STASH: *mut Vec<u32> = std::ptr::null_mut();

fn stash(v: &mut Vec<u32>) {
    unsafe { STASH = v as *mut Vec<u32> };
}

#[inline(never)]
fn clear_it() {
    unsafe { (*STASH).clear() };
}

/// Shown, it clears what the pointer kept points to.
struct Clears;

impl std::fmt::Display for Clears {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        clear_it();
        write!(f, "cleared")
    }
}

fn each(f: &mut dyn FnMut()) {
    f();
}

fn main() {
    // A function given the `&mut`.
    let mut v: Vec<u32> = vec![1, 2, 3];
    stash(&mut v);
    if v.len() > 2 {
        clear_it();
        println!("{}", (v.len() - 3) as u32);
    }
    // A closure that holds the `&mut`, called.
    let mut w: Vec<u32> = vec![1, 2, 3];
    let mut take = || unsafe { STASH = &mut w as *mut Vec<u32> };
    take();
    if w.len() > 2 {
        clear_it();
        println!("{}", (w.len() - 3) as u32);
    }
    // A closure given to a call as a `&mut dyn FnMut`.
    let mut d: Vec<u32> = vec![1, 2, 3];
    each(&mut || unsafe { STASH = &mut d as *mut Vec<u32> });
    if d.len() > 2 {
        clear_it();
        println!("{}", (d.len() - 3) as u32);
    }
    // Std's code that gives back a raw pointer to it, `ptr::from_mut`
    // and `NonNull::from`: kept, and written through past the test.
    let mut m: Vec<u32> = vec![1, 2, 3];
    let p = std::ptr::from_mut(&mut m);
    unsafe { STASH = p };
    if m.len() > 2 {
        clear_it();
        println!("{}", (m.len() - 3) as u32);
    }
    let mut n: Vec<u32> = vec![1, 2, 3];
    let q = std::ptr::NonNull::from(&mut n);
    unsafe { STASH = q.as_ptr() };
    if n.len() > 2 {
        clear_it();
        println!("{}", (n.len() - 3) as u32);
    }
    // Std's code past the test that runs the crate's: a `Display` impl
    // that `format!` runs, and a closure `Option::map` runs.
    let mut e: Vec<u32> = vec![1, 2, 3];
    stash(&mut e);
    if e.len() > 2 {
        let shown = format!("{}", Clears);
        println!("{shown} {}", (e.len() - 3) as u32);
    }
    let mut o: Vec<u32> = vec![1, 2, 3];
    stash(&mut o);
    if o.len() > 2 {
        let one = Some(1u32).map(|x| {
            clear_it();
            x
        });
        println!("{one:?} {}", (o.len() - 3) as u32);
    }
    // Formatting made before the test, run past it: what `format_args!`
    // holds runs the `Display` impl only where it's formatted.
    let mut a: Vec<u32> = vec![1, 2, 3];
    stash(&mut a);
    let later = format_args!("{}", Clears);
    if a.len() > 2 {
        let shown = std::fmt::format(later);
        println!("{shown} {}", (a.len() - 3) as u32);
    }
    // Std's code given the kept pointer itself, which writes through it
    // running none of the crate's: `(*STASH).truncate(0)`, `ptr::replace`.
    let mut t: Vec<u32> = vec![1, 2, 3];
    stash(&mut t);
    if t.len() > 2 {
        unsafe { (*STASH).truncate(0) };
        println!("{}", (t.len() - 3) as u32);
    }
    let mut w2: Vec<u32> = vec![1, 2, 3];
    stash(&mut w2);
    if w2.len() > 2 {
        drop(unsafe { std::ptr::replace(STASH, Vec::new()) });
        println!("{}", (w2.len() - 3) as u32);
    }
    // Written again whole: what the test shows holds, no pointer reaching it.
    let mut r: Vec<u32> = vec![1];
    stash(&mut r);
    r = vec![5, 6, 7];
    if r.len() > 2 {
        println!("{}", r[2]);
    }
    // An index past the call.
    v.push(9);
    stash(&mut v);
    if !v.is_empty() {
        clear_it();
        println!("{}", v[0]);
    }
}
