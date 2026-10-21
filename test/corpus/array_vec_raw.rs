// A vector of a fixed capacity written as arrayvec writes it: its items a
// `[MaybeUninit<T>; CAP]`, reached through raw pointers, `ptr::copy` to
// make room or close a gap, and `slice::from_raw_parts` for its slice. Each
// pointer is a handle on an item of the array (ADR 0407).
use std::mem::{self, MaybeUninit};
use std::ptr;
use std::slice;

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
}

struct Stack<T, const CAP: usize> {
    xs: [MaybeUninit<T>; CAP],
    len: usize,
}

// Its capacity, a trait's constant of each `CAP`: `Self::CAPACITY` in
// generic code is the impl's, `CAP`.
trait Capacity {
    const CAPACITY: usize;
}

impl<T, const CAP: usize> Capacity for Stack<T, CAP> {
    const CAPACITY: usize = CAP;
}

/// `n` items along, by bytes for a type with no size, as arrayvec steps.
fn step<T>(p: *mut T, n: usize) -> *mut T {
    if mem::size_of::<T>() == 0 {
        p.cast::<u8>().wrapping_add(n).cast::<T>()
    } else {
        unsafe { p.add(n) }
    }
}

impl<T, const CAP: usize> Stack<T, CAP> {
    fn is_full(&self) -> bool {
        self.len == Self::CAPACITY
    }

    /// Of a whole array: its items, read as `MaybeUninit`s.
    fn from_array(array: [T; CAP]) -> Self {
        let xs = unsafe { ptr::read(&array as *const [T; CAP] as *const [MaybeUninit<T>; CAP]) };
        mem::forget(array);
        Stack { xs, len: CAP }
    }

    /// Its items, read as the whole array, when it's full.
    fn into_array(self) -> [T; CAP] {
        assert!(self.is_full());
        let array = unsafe { ptr::read(self.as_ptr() as *const [T; CAP]) };
        mem::forget(self);
        array
    }

    fn new() -> Self {
        Stack { xs: [const { MaybeUninit::uninit() }; CAP], len: 0 }
    }

    fn as_ptr(&self) -> *const T {
        self.xs.as_ptr() as *const T
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        self.xs.as_mut_ptr() as *mut T
    }

    fn push(&mut self, value: T) {
        assert!(self.len < CAP);
        unsafe { step(self.as_mut_ptr(), self.len).write(value) };
        self.len += 1;
    }

    fn insert(&mut self, index: usize, value: T) {
        assert!(index <= self.len && self.len < CAP);
        unsafe {
            let p = self.as_mut_ptr().add(index);
            ptr::copy(p, p.add(1), self.len - index);
            ptr::write(p, value);
        }
        self.len += 1;
    }

    fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len);
        unsafe {
            let p = self.as_mut_ptr().add(index);
            let taken = ptr::read(p);
            ptr::copy(p.add(1), p, self.len - index - 1);
            self.len -= 1;
            taken
        }
    }

    /// Copies of `other`'s items after its own, read through the shared
    /// slice's pointer.
    fn extend_from_slice(&mut self, other: &[T])
    where
        T: Copy,
    {
        assert!(self.len + other.len() <= CAP);
        unsafe { ptr::copy_nonoverlapping(other.as_ptr(), self.as_mut_ptr().add(self.len), other.len()) };
        self.len += other.len();
    }

    /// Its last items dropped, in place, till it has `len`.
    fn truncate(&mut self, len: usize) {
        while self.len > len {
            self.len -= 1;
            unsafe { ptr::drop_in_place(self.as_mut_ptr().add(self.len)) };
        }
    }

    fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }
}

fn main() {
    let mut numbers: Stack<i32, 8> = Stack::new();
    for n in [5, 1, 4] {
        numbers.push(n);
    }
    numbers.insert(1, 9);
    println!("{:?}", numbers.as_slice());
    println!("{}", numbers.remove(0));
    numbers.as_mut_slice().sort();
    let more = [20, 30, 40];
    numbers.extend_from_slice(&more[1..]);
    println!("{:?} {}", numbers.as_slice(), numbers.len);
    let mut words: Stack<String, 4> = Stack::new();
    words.push("b".to_string());
    words.insert(0, "a".to_string());
    words.as_mut_slice()[1].push('!');
    println!("{:?}", words.as_slice());
    let mut copied = [0u8; 6];
    let source = [1u8, 2, 3];
    unsafe { ptr::copy_nonoverlapping(source.as_ptr(), copied.as_mut_ptr().add(2), 3) };
    unsafe { copied.as_mut_ptr().copy_from(source.as_ptr().add(1), 2) };
    println!("{copied:?}");
    // An array's pointer as its first item's.
    let row = [4, 5, 6];
    let first = (&row as *const [i32; 3]).cast::<i32>();
    println!("{}", unsafe { *first.add(2) });
    let mut noisy: Stack<Noisy, 3> = Stack::new();
    noisy.push(Noisy("a"));
    noisy.push(Noisy("b"));
    noisy.truncate(0);
    println!("{}", noisy.len);
    let full: Stack<i32, 3> = Stack::from_array([7, 8, 9]);
    println!("{} {:?}", full.is_full(), full.into_array());
    // A `Copy` value JS holds as an object, which a copy changes: each
    // place its own, as Rust's copy is.
    let mut points: Stack<Point, 4> = Stack::new();
    points.push(Point { x: 1 });
    points.push(Point { x: 2 });
    let mut moved = [Point { x: 0 }; 2];
    unsafe { ptr::copy_nonoverlapping(points.as_ptr(), moved.as_mut_ptr(), 2) };
    moved[0].x = 10;
    println!("{:?} {:?}", points.as_slice(), moved);
    // Storage held as one `MaybeUninit<[T; N]>`, as heapless holds it: a
    // pointer to it cast to a `T`'s is its first item's.
    let mut storage = MaybeUninit::new([0u32; 3]);
    let slot = (&raw mut storage).cast::<u32>();
    unsafe {
        slot.write(5);
        slot.add(1).write(6);
        slot.add(2).write(7);
    }
    println!("{:?}", unsafe { storage.assume_init() });
}
