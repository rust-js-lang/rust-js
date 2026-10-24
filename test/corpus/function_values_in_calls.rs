// A function as a value, `.map(show)`, is an arrow lowered in an evaluation
// of its own: what's made around it, the cleanup that unwinds to, and the
// copies back a call around it owes stay the enclosing call's, and its own
// `&mut`s are copied back in the arrow.

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn bump(x: &mut u32) {
    *x += 1;
}

fn show(l: &Loud) -> usize {
    l.0.len()
}

fn twice(n: &mut u32, f: fn(&mut u32)) -> u32 {
    f(n);
    f(n);
    *n
}

fn tally(total: &mut u32, lens: Vec<usize>) {
    *total += lens.iter().sum::<usize>() as u32;
}

trait Step {
    fn add(&mut self, by: i32);
    fn now(&self) -> i32;
    fn step(&mut self, by: i32) -> i32 {
        self.add(by);
        self.now()
    }
}

impl Step for i32 {
    fn add(&mut self, by: i32) {
        *self += by;
    }
    fn now(&self) -> i32 {
        *self
    }
}

fn apply(n: &mut i32, f: fn(&mut i32, i32) -> i32) -> i32 {
    f(n, 10)
}

fn sum_into(total: &mut u32, items: Vec<u32>) {
    *total += items.iter().sum::<u32>();
}

fn main() {
    let mut x = 1;
    let mut y = 2;
    println!("{} {x}", apply(&mut x, <i32 as Step>::step) + apply(&mut y, <i32 as Step>::step));
    println!("{y}");
    let mut ys = vec![4u32, 5];
    let mut acc = 1;
    sum_into(&mut acc, ys.iter_mut().map(std::mem::take).collect());
    println!("{acc} {ys:?}");
    let mut k = 7u32;
    let taken: Vec<u32> = [&mut k].into_iter().map(std::mem::take).collect();
    println!("{taken:?} {k}");
    let mut xs = vec![1u32, 2, 3];
    xs.iter_mut().for_each(bump);
    println!("{xs:?}");
    let mut n = 5;
    println!("{}", twice(&mut n, bump));
    println!("{n}");
    let mut total = 0;
    tally(&mut total, [Loud("a"), Loud("bb")].iter().map(show).collect());
    println!("{total}");
    let held = Loud("held");
    let lens: Vec<usize> = [Loud("c"), Loud("ddd")].iter().map(show).chain([show(&held)]).collect();
    println!("{lens:?}");
    let mut m = 1;
    let got = twice(&mut m, bump) + [Loud("e")].iter().map(show).sum::<usize>() as u32;
    println!("{got} {m}");
}
