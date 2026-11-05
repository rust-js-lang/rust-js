// Generic code's `for` loop over the crate's own generic iterator struct,
// `for x in w` of a `Wrapper<I>`, steps it by its own `next`, as std's
// blanket `impl<I: Iterator> IntoIterator for I` gives it back as it is
// (#806). So does an impl's method of `impl<I: Iterator> Trait for
// Wrapper<I>` looping `for x in self`, and a trait's default copied into
// one, in the impl's terms (#803).

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Wrapper<I>(I, &'static str);

impl<I: Iterator> Iterator for Wrapper<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let item = self.0.next();
        println!("{} next: {}", self.1, item.is_some());
        item
    }
}

impl<I> Drop for Wrapper<I> {
    fn drop(&mut self) {
        println!("drop {}", self.1);
    }
}

fn looped<I: Iterator>(w: Wrapper<I>) -> usize {
    let mut n = 0;
    for _ in w {
        n += 1;
    }
    n
}

fn dropping<I: Iterator<Item = u32>>(w: Wrapper<I>) -> u32 {
    let mut sum = 0;
    for n in w {
        let d = D(n);
        if n == 2 {
            break;
        }
        sum += d.0;
    }
    sum
}

fn of_items<T, I: Iterator<Item = T>>(w: Wrapper<I>) -> Vec<T> {
    let mut all = Vec::new();
    for item in w {
        all.push(item);
    }
    all
}

trait Counted {
    fn counted(self) -> usize;
}

impl<I: Iterator> Counted for Wrapper<I> {
    fn counted(self) -> usize {
        let mut n = 0;
        for _ in self {
            n += 1;
        }
        n
    }
}

trait Tallying: Iterator {
    fn tally(self) -> usize
    where
        Self: Sized,
    {
        let mut n = 0;
        for _ in self {
            n += 1;
        }
        n
    }
}

impl<I: Iterator> Tallying for Wrapper<I> {}

fn main() {
    println!("{}", looped(Wrapper(0..3, "range")));
    println!("{}", dropping(Wrapper(vec![1, 2, 3].into_iter(), "vec")));
    let items = of_items(Wrapper(vec![D(7), D(8)].into_iter(), "items"));
    println!("{}", items.len());
    println!("{}", Wrapper("ab".chars(), "counted").counted());
    println!("{}", Wrapper(vec![D(9)].into_iter(), "tally").tally());
}
