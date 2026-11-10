// A `&mut Box<dyn FnMut>` made a `&mut dyn FnMut`: the function the box
// holds, called through the reference, its state kept between calls, and
// the box dropped after, its captured value's drop with it (#1222). The
// box JS holds in a cell, as a `&mut` of it is lent (ADR 1086); the `dyn
// FnMut` is the function in it.
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn run(f: &mut dyn FnMut(i32)) {
    f(1);
    f(2);
}

fn total(f: &mut dyn FnMut(i32) -> i32, from: i32) -> i32 {
    f(from) + f(from + 1)
}

fn read(f: &mut dyn Fn() -> String) -> String {
    f()
}

fn main() {
    let mut sum = 0;
    let mut b: Box<dyn FnMut(i32)> = Box::new(|x| sum += x);
    run(&mut b);
    run(&mut b);
    drop(b);
    println!("{sum}");

    let held = Noisy("the counter's");
    let mut count = 0;
    let mut counter: Box<dyn FnMut(i32) -> i32> = Box::new(move |x| {
        let _ = &held;
        count += 1;
        x * count
    });
    println!("{}", total(&mut counter, 3));
    println!("{}", total(&mut counter, 10));
    counter = Box::new(|x| x - 1);
    println!("{}", total(&mut counter, 5));
    drop(counter);

    let name = String::from("fig");
    let mut shown: Box<dyn Fn() -> String> = Box::new(move || name.clone());
    println!("{}", read(&mut shown));
    println!("end");
}
