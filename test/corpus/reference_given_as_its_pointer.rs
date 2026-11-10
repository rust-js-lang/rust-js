// A function whose signature has a raw pointer in its result may let a
// reference it's given out as that pointer: it's given the pointer to the
// place, never a box of a copy, and a pointer of a type parameter is a
// handle on its place in generic code, a slice's adapted at the call, in
// each call shape (#1143, ADR 1114).
use std::mem::MaybeUninit;

fn ptr_from_mut<T: ?Sized>(r: &mut T) -> *mut T {
    r
}

fn ptr_from_ref<T: ?Sized>(r: &T) -> *const T {
    r
}

// A pointer let out through another parameter.
fn store_in<T>(r: &mut T, out: &mut *mut T) {
    *out = r;
}

// A raw pointer given by value takes nothing out: `count`, a box this
// function was given, is given on as it is.
fn note<T>(_seen: *const T, count: &mut u32) {
    *count += 1;
}

fn noted(count: &mut u32) {
    let local = 3u8;
    note(&raw const local, count);
}

fn first_of<T>(p: *mut [T]) -> *mut T {
    unsafe { (*p).as_mut_ptr() }
}

fn same<T: ?Sized>(p: *mut T) -> *mut T {
    p
}

fn std_from_mut<T: ?Sized>(r: &mut T) -> *mut T {
    std::ptr::from_mut(r)
}

fn std_from_ref<T: ?Sized>(r: &T) -> *const T {
    std::ptr::from_ref(r)
}

// Generic code's own `&T` given on, of a type only its callers know.
fn from_ref_on<T: ?Sized>(r: &T) -> *const T {
    ptr_from_ref(r)
}

/// # Safety
/// Each item of `slice` is written.
unsafe fn slice_assume_init_mut<T>(slice: &mut [MaybeUninit<T>]) -> &mut [T] {
    let ptr = ptr_from_mut::<[MaybeUninit<T>]>(slice) as *mut [T];
    unsafe { &mut *ptr }
}

fn slice_as_uninit_mut<T>(slice: &mut [T]) -> &mut [MaybeUninit<T>] {
    let ptr = ptr_from_mut::<[T]>(slice) as *mut [MaybeUninit<T>];
    unsafe { &mut *ptr }
}

fn slice_as_uninit<T>(slice: &[T]) -> &[MaybeUninit<T>] {
    let ptr = ptr_from_ref::<[T]>(slice) as *const [MaybeUninit<T>];
    unsafe { &*ptr }
}

// The caller's own type parameter, given on.
fn bump_through<T: Copy + std::ops::Add<Output = T>>(slot: &mut T, by: T) -> *mut T {
    let p = ptr_from_mut(slot);
    unsafe { *p = *p + by };
    p
}

trait Pointed {
    fn pointer(&mut self) -> *mut Self;
}

impl Pointed for u32 {
    fn pointer(&mut self) -> *mut Self {
        self
    }
}

impl Pointed for Vec<u8> {
    fn pointer(&mut self) -> *mut Self {
        self
    }
}

fn through_bound<P: Pointed>(p: &mut P) -> *mut P {
    p.pointer()
}

trait Slot {
    fn slot(&mut self) -> *mut u32;
}

impl Slot for Counter {
    fn slot(&mut self) -> *mut u32 {
        &mut self.n
    }
}

// A closure given to generic code, which calls it of its own `&mut T`.
fn given_on<T, F: FnMut(&mut T) -> *mut T>(slot: &mut T, mut f: F) -> *mut T {
    f(slot)
}

struct Counter {
    n: u32,
    label: String,
}

fn main() {
    // A slice's items as `MaybeUninit`s and back.
    let mut bytes = [1u8, 2, 3];
    let uninit = slice_as_uninit_mut(&mut bytes);
    uninit[1] = MaybeUninit::new(20);
    let back = unsafe { slice_assume_init_mut(uninit) };
    back[2] = 30;
    println!("{bytes:?} {}", slice_as_uninit(&bytes).len());
    println!("{}", unsafe { (&(*from_ref_on(&bytes[..]))).len() });

    // A number's, written after the call, and the same pointer `&raw mut` is.
    let mut n = 5u32;
    let p = ptr_from_mut(&mut n);
    unsafe { *p += 1 };
    println!("{n} {}", std::ptr::eq(p, &raw mut n));

    // An object's.
    let mut words = vec![String::from("a"), String::from("b")];
    let q = ptr_from_mut(&mut words);
    unsafe { (*q).push(String::from("c")) };
    println!("{words:?} {}", std::ptr::eq(q, &raw mut words));

    // Through another parameter.
    let mut t = 1u8;
    // A raw pointer given by value takes nothing out: `ptr::hash`'s
    // hasher is given as a `&mut` is.
    let mut hasher = std::hash::DefaultHasher::new();
    std::ptr::hash(&raw const t, &mut hasher);
    let mut seen = 0u32;
    noted(&mut seen);
    println!("hashed {seen}");
    let mut out = std::ptr::null_mut();
    store_in(&mut t, &mut out);
    unsafe { *out += 1 };
    println!("{t}");

    // std's own, the polyfill's one path, in generic code and here.
    let m = std_from_mut(&mut n);
    unsafe { *m *= 10 };
    let mut k = 1u32;
    let direct = std::ptr::from_mut(&mut k);
    unsafe { *direct += 1 };
    println!("{n} {} {k} {}", std::ptr::eq(m, &raw mut n), std::ptr::eq(direct, &raw mut k));
    let read = std::ptr::from_ref(&k);
    println!("{} {}", unsafe { *read }, std::ptr::eq(read, &raw const k));
    let w = std_from_mut(&mut words);
    unsafe { (*w).push(String::from("d")) };
    let s = std_from_mut(&mut bytes[..]);
    unsafe { (*s)[0] = 9 };
    println!("{words:?} {bytes:?} {}", unsafe { (&(*std_from_ref(&bytes[..]))).len() });

    // A field's, and an item's.
    let mut counter = Counter {
        n: 1,
        label: String::from("c"),
    };
    let field = ptr_from_mut(&mut counter.n);
    unsafe { *field += 10 };
    let mut items = [7u16, 8];
    let item = ptr_from_mut(&mut items[1]);
    unsafe { *item *= 2 };
    println!("{} {} {items:?}", counter.n, counter.label);

    // A slice's pointer given to and back from generic code.
    let mut more = vec![4u8, 5, 6];
    let whole = same(&mut more[..] as *mut [u8]);
    unsafe { (*whole)[0] = 40 };
    let head = first_of(&mut more[..] as *mut [u8]);
    unsafe { *head += 1 };
    println!("{more:?}");

    // The caller's own type parameter.
    let mut k = 2i64;
    let kp = bump_through(&mut k, 3);
    unsafe { *kp *= 2 };
    println!("{k}");

    // Through a bound's dictionary.
    let mut m = 9u32;
    let mp = through_bound(&mut m);
    unsafe { *mp += 1 };
    let mut v = vec![1u8];
    let vp = through_bound(&mut v);
    unsafe { (*vp).push(2) };
    println!(
        "{m} {v:?} {} {}",
        std::ptr::eq(mp, &raw mut m),
        std::ptr::eq(vp, &raw mut v)
    );
    // An impl's method called of its own type, an object's and a number's.
    let own = v.pointer();
    unsafe { (*own).push(3) };
    let own_n = m.pointer();
    unsafe { *own_n += 1 };
    println!("{m} {v:?}");

    // Through a function pointer.
    let f: fn(&mut u32) -> *mut u32 = ptr_from_mut;
    let mut z = 100u32;
    let zp = f(&mut z);
    unsafe { *zp += 1 };
    println!("{z} {}", std::ptr::eq(zp, &raw mut z));

    // Through a closure, and a `dyn`.
    let g = |r: &mut u32| -> *mut u32 { r };
    let mut y = 50u32;
    let yp = g(&mut y);
    unsafe { *yp += 1 };
    println!("{y} {}", std::ptr::eq(yp, &raw mut y));
    let mut x = 70u32;
    let xp = given_on(&mut x, |r| r);
    unsafe { *xp += 1 };
    let mut list = vec![1u8];
    let lp = given_on(&mut list, |r| r);
    unsafe { (*lp).push(2) };
    println!("{x} {} {list:?}", std::ptr::eq(xp, &raw mut x));
    let mut held = Counter {
        n: 3,
        label: String::from("d"),
    };
    let dyn_slot: &mut dyn Slot = &mut held;
    let dp = dyn_slot.slot();
    unsafe { *dp += 1 };
    let same_slot = std::ptr::eq(dp, &raw mut held.n);
    println!("{} {} {same_slot}", held.n, held.label);
}
