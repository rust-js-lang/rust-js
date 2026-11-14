// A raw pointer a function keeps of a `&mut` it's given writes its caller's
// place after the call (#1781): the `&mut` is given as the place's handle,
// not a box copied back, which the pointer's later write wouldn't reach,
// and so where it's given on to the function that keeps it, however many
// calls down, through a function pointer or a closure, either may be
// one, or through a trait's method generic code or a `dyn` calls, which
// any of its impls may be, or by code std runs of what it's given: a
// closure, a function, a function pointer, a `dyn`, a type's `Drop`; and
// a pointer std's code makes of it, `ptr::from_mut`, a slice's
// `as_mut_ptr`, a `NonNull` kept in a field, is one it keeps.
// Miri finds no undefined behaviour: a read through the owner, `n == 0`,
// leaves the pointer it lent able to write.
use std::cell::Cell;
use std::ptr::NonNull;

thread_local! {
    static KEPT: Cell<*mut u32> = const { Cell::new(std::ptr::null_mut()) };
    static HELD: Cell<Option<Pointed>> = const { Cell::new(None) };
}

fn keep(n: &mut u32) {
    KEPT.with(|kept| kept.set(n));
}

#[derive(Clone, Copy)]
struct Pointed {
    to: NonNull<u32>,
}

fn zero_held() {
    HELD.with(|held| unsafe { *held.get().unwrap().to.as_ptr() = 0 });
}

fn kept_of_std_from_mut(n: &mut u32) {
    KEPT.with(|kept| kept.set(std::ptr::from_mut(n)));
}

fn kept_of_a_slices_pointer(n: &mut u32) {
    KEPT.with(|kept| kept.set(std::slice::from_mut(n).as_mut_ptr()));
}

fn kept_in_a_field(n: &mut u32) {
    HELD.with(|held| held.set(Some(Pointed { to: NonNull::from(n) })));
}

fn zero_kept() {
    KEPT.with(|kept| unsafe { *kept.get() = 0 });
}

#[inline(never)]
fn apply<F: FnOnce()>(f: F) {
    f();
}

fn kept_in_a_closure(n: &mut u32) {
    apply(move || KEPT.with(|kept| kept.set(n)));
}

trait GetMut {
    fn get_mut(&mut self) -> &mut u32;
}

impl GetMut for u32 {
    fn get_mut(&mut self) -> &mut u32 {
        self
    }
}

fn kept_by_generic_code(n: &mut impl GetMut) {
    let n = n.get_mut();
    KEPT.with(|kept| kept.set(n));
}

fn deep1(n: &mut u32) {
    deep2(n);
}

fn deep2(n: &mut u32) {
    deep3(n);
}

fn deep3(n: &mut u32) {
    deep4(n);
}

fn deep4(n: &mut u32) {
    deep5(n);
}

fn deep5(n: &mut u32) {
    keep(n);
}

fn through_a_pointer(n: &mut u32, f: fn(&mut u32)) {
    f(n);
}

fn through_a_closure<F: FnMut(&mut u32)>(n: &mut u32, mut f: F) {
    f(n);
}

trait Keeper {
    fn take(&self, n: &mut u32);
}

struct Keeps;

impl Keeper for Keeps {
    fn take(&self, n: &mut u32) {
        keep(n);
    }
}

fn through_a_bound<K: Keeper>(n: &mut u32, k: &K) {
    k.take(n);
}

fn through_a_dyn(n: &mut u32, k: &dyn Keeper) {
    k.take(n);
}

trait Runs {
    fn run(&self, n: &mut u32);
}

struct Calls<F>(F);

impl<F: Fn(&mut u32)> Runs for Calls<F> {
    fn run(&self, n: &mut u32) {
        (self.0)(n);
    }
}

fn through_a_dyn_of_a_closure(n: &mut u32, r: &dyn Runs) {
    r.run(n);
}

fn kept_by_std_given_a_closure(n: &mut u32) {
    std::iter::once(()).for_each(|()| KEPT.with(|kept| kept.set(&mut *n)));
}

fn kept_by_std_given_a_function(n: &mut u32) {
    std::iter::once(n).for_each(keep);
}

fn kept_by_std_given_a_pointer(n: &mut u32) {
    std::iter::once(n).for_each(keep as fn(&mut u32));
}

struct Holder<'a>(&'a mut u32);

impl Drop for Holder<'_> {
    fn drop(&mut self) {
        keep(&mut *self.0);
    }
}

fn kept_when_dropped(n: &mut u32) {
    let _held = Holder(n);
}

fn passed_on(n: &mut u32) {
    keep(n);
}

fn add_one(n: &mut u32) {
    *n += 1;
}

fn stashed(mut n: u32) -> u32 {
    keep(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_by_another(mut n: u32) -> u32 {
    passed_on(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_in_a_closure(mut n: u32) -> u32 {
    kept_in_a_closure(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_by_generic_code(mut n: u32) -> u32 {
    kept_by_generic_code(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_deep(mut n: u32) -> u32 {
    deep1(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_through_a_pointer(mut n: u32) -> u32 {
    through_a_pointer(&mut n, keep);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_through_a_closure(mut n: u32) -> u32 {
    through_a_closure(&mut n, |n| keep(n));
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_through_a_bound<K: Keeper>(mut n: u32, k: &K) -> u32 {
    through_a_bound(&mut n, k);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_through_a_dyn(mut n: u32) -> u32 {
    through_a_dyn(&mut n, &Keeps);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_by_std_given_a_closure(mut n: u32) -> u32 {
    kept_by_std_given_a_closure(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_by_std_given_a_function(mut n: u32) -> u32 {
    kept_by_std_given_a_function(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_when_dropped(mut n: u32) -> u32 {
    kept_when_dropped(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_through_a_dyn_of_a_closure(mut n: u32) -> u32 {
    through_a_dyn_of_a_closure(&mut n, &Calls(|n: &mut u32| keep(n)));
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_by_std_given_a_pointer(mut n: u32) -> u32 {
    kept_by_std_given_a_pointer(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_through_a_boxed_dyn(mut n: u32) -> u32 {
    through_a_closure(&mut n, Box::new(|n: &mut u32| keep(n)) as Box<dyn FnMut(&mut u32)>);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_of_std_from_mut(mut n: u32) -> u32 {
    kept_of_std_from_mut(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_of_a_slices_pointer(mut n: u32) -> u32 {
    kept_of_a_slices_pointer(&mut n);
    if n == 0 {
        return 0;
    }
    zero_kept();
    n - 1
}

fn stashed_in_a_field(mut n: u32) -> u32 {
    kept_in_a_field(&mut n);
    if n == 0 {
        return 0;
    }
    zero_held();
    n - 1
}

fn bumped(mut n: u32) -> u32 {
    add_one(&mut n);
    n
}

fn main() {
    println!("{}", stashed(5));
    println!("{}", stashed_by_another(7));
    println!("{}", stashed_in_a_closure(9));
    println!("{}", stashed_by_generic_code(11));
    println!("{}", stashed_deep(13));
    println!("{}", stashed_through_a_pointer(15));
    println!("{}", stashed_through_a_closure(17));
    println!("{}", stashed_through_a_bound(19, &Keeps));
    println!("{}", stashed_through_a_dyn(21));
    println!("{}", stashed_by_std_given_a_closure(23));
    println!("{}", stashed_by_std_given_a_function(25));
    println!("{}", stashed_when_dropped(27));
    println!("{}", stashed_through_a_dyn_of_a_closure(29));
    println!("{}", stashed_by_std_given_a_pointer(31));
    println!("{}", stashed_through_a_boxed_dyn(33));
    println!("{}", stashed_of_std_from_mut(35));
    println!("{}", stashed_of_a_slices_pointer(37));
    println!("{}", stashed_in_a_field(39));
    println!("{}", bumped(5));
}
