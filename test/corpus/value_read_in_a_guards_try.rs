// A value made before a guard's `try` is read where it's used, in the
// `try`, where it can't throw: entering a `try` runs nothing, and its
// `catch` can't run of it (rd233). One that may throw stays where it was
// made, before the `try`, as a panic in it drops nothing yet.

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Done;

impl Drop for Done {
    fn drop(&mut self) {
        println!("done");
    }
}

fn after(n: u32, _done: &Done) -> u32 {
    println!("after {n}");
    if n > 10 {
        panic!("too late");
    }
    n
}

struct Pair {
    left: u32,
    right: u32,
}

fn made(fail: bool) -> u32 {
    if fail {
        panic!("made failed");
    }
    4
}

fn take(n: u32, guard: &Noisy) -> u32 {
    println!("take {n} {}", guard.0);
    if n > 10 {
        panic!("too big");
    }
    n + 1
}

fn count(n: u32) -> u32 {
    println!("count {n}");
    if n > 10 {
        panic!("too many");
    }
    n * 2
}

// What can't throw, read in the call it's given to.
fn summed(pair: &Pair, by: u32) -> u32 {
    let _guard = Noisy("summed");
    count(pair.left.wrapping_mul(by) ^ pair.right)
}

// What may throw, made before the guard, where a panic in it drops
// nothing.
fn first_made(fail: bool) -> u32 {
    take(made(fail), &Noisy("guard"))
}

// And one made before the guard, where a panic in it drops nothing.
fn guarded_after(fail: bool) -> u32 {
    after(made(fail), &Done)
}

fn shown(noisy: &Noisy, by: u32) -> u32 {
    println!("shown {}", noisy.0);
    after(by, &Done)
}

// And one the `try`'s cleanup drops: held before it, as its `finally`
// reads it.
fn held(by: u32) -> u32 {
    shown(&Noisy("held"), by)
}

fn main() {
    let pair = Pair { left: 2, right: 3 };
    for by in [1, 3] {
        println!("{:?}", std::panic::catch_unwind(|| summed(&pair, by)).ok());
        println!("{:?}", std::panic::catch_unwind(|| held(by * 5)).ok());
    }
    for fail in [false, true] {
        println!("{:?}", std::panic::catch_unwind(|| first_made(fail)).ok());
        println!("{:?}", std::panic::catch_unwind(|| guarded_after(fail)).ok());
    }
}
