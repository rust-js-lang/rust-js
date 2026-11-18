//@ run-fail: index out of bounds: the len is 0 but the index is 0
// A `Vec` lent `&mut` through a `dyn`, to code that keeps a raw pointer to
// what the `dyn`'s method gives out, or to the `dyn` itself: a later call
// may change its length through the pointer, so a test of its length holds
// only until a call (#2053, as #1966 of a `&mut Vec`). `len() - 3` past
// `len() > 2` wraps where the call cleared it, and `v[0]` panics.
trait Items {
    fn items(&mut self) -> &mut Vec<u32>;
}

impl Items for Vec<u32> {
    fn items(&mut self) -> &mut Vec<u32> {
        self
    }
}

static mut STASH: *mut Vec<u32> = std::ptr::null_mut();
static mut HELD: Option<*mut dyn Items> = None;

/// Keeps a pointer to the `Vec` the `dyn` gives out.
fn stash(v: &mut dyn Items) {
    let items: &mut Vec<u32> = v.items();
    unsafe { STASH = items as *mut Vec<u32> };
}

/// Keeps a pointer to the `dyn` itself.
fn hold(v: &mut (dyn Items + 'static)) {
    unsafe { HELD = Some(v as *mut dyn Items) };
}

#[inline(never)]
fn clear_it() {
    unsafe { (*STASH).clear() };
}

#[inline(never)]
fn clear_held() {
    unsafe {
        if let Some(held) = HELD {
            (*held).items().clear();
        }
    }
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let _loud = Loud(1);
    let mut v: Vec<u32> = vec![1, 2, 3];
    stash(&mut v);
    if v.len() > 2 {
        clear_it();
        println!("{}", (v.len() - 3) as u32);
    }
    let mut w: Vec<u32> = vec![4, 5, 6];
    hold(&mut w);
    if w.len() > 2 {
        clear_held();
        println!("{}", (w.len() - 3) as u32);
    }
    let mut kept: Vec<u32> = vec![7, 8, 9];
    stash(&mut kept);
    if kept.len() > 2 {
        println!("{}", kept[2]);
    }
    v.push(9);
    stash(&mut v);
    if !v.is_empty() {
        clear_it();
        println!("{}", v[0]);
    }
}
