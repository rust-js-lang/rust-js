// A C-variadic function's last parameter gathers what it's given past the
// rest, `...args`, the array of it: `next_arg()` is the first of what's
// left, `shift()`, and a clone of what's left reads on its own (ADR 0428).
// Of a function, a method, and through a function pointer, a trait's
// `&mut self` one's too, whose arrow gathers the rest and passes it on.
unsafe extern "C" fn sum(count: i32, mut args: ...) -> i64 {
    let mut total = 0i64;
    for _ in 0..count {
        total += unsafe { args.next_arg::<i64>() };
    }
    total
}

unsafe extern "C" fn first_twice(mut args: ...) -> (i32, i32, i32) {
    let mut again = args.clone();
    let a = unsafe { args.next_arg::<i32>() };
    let b = unsafe { again.next_arg::<i32>() };
    let c = unsafe { args.next_arg::<i32>() };
    (a, b, c)
}

struct Scale(f64);

impl Scale {
    unsafe extern "C" fn apply(&self, mut args: ...) -> f64 {
        self.0 * unsafe { args.next_arg::<f64>() }
    }
}

trait Counter: Sized {
    fn get(&self) -> i32;

    unsafe extern "C" fn bump(&mut self, mut args: ...) -> i32 {
        self.get() + unsafe { args.next_arg::<i32>() }
    }
}

struct Count(i32);

impl Counter for Count {
    fn get(&self) -> i32 {
        self.0
    }
}

fn main() {
    unsafe {
        println!("{} {}", sum(3, 1i64, 2i64, 3i64), sum(0));
        println!("{:?}", first_twice(7, 8, 9));
        println!("{}", Scale(2.5).apply(4.0f64));
        let pointer: unsafe extern "C" fn(i32, ...) -> i64 = sum;
        println!("{}", pointer(2, 10i64, 20i64));
        let bump = Count::bump as unsafe extern "C" fn(&mut Count, ...) -> i32;
        let mut count = Count(100);
        println!("{} {}", bump(&mut count, 32), count.0);
    }
}
