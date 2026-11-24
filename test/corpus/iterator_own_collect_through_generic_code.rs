//@ library-differs
// An iterator of the crate's whose impl writes its own `collect` runs it
// where generic code collects it, as Rust's dispatch picks: its own builds
// what it's asked for another way, here counting what it's given first;
// std's `collect` runs through a `&mut` of it and of a `map` of it, as
// std's impls for those pass none on (#2430). A trait's default in a
// blanket impl that collects `Self` into a `Vec<Self::Item>`, itertools'
// `collect_vec`, dispatches too, its items' layout its impl's makers'.
struct Counter(u32);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 <= 3).then_some(self.0)
    }
    fn collect<B: FromIterator<u32>>(self) -> B {
        println!("own collect from {}", self.0);
        std::iter::once(99).collect()
    }
}

trait Tools: Iterator + Sized {
    fn all(self) -> Vec<Self::Item> {
        self.collect()
    }
}

impl<I: Iterator> Tools for I {}

fn gathered<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    it.collect()
}

fn gathered_set<I: Iterator<Item = u32>>(it: I) -> std::collections::BTreeSet<u32> {
    it.collect()
}

fn main() {
    println!("{:?}", gathered(Counter(0)));
    println!("{:?}", gathered_set(Counter(0)));
    println!("{:?}", gathered(&mut Counter(0)));
    println!("{:?}", gathered(Counter(0).map(|n| n * 10)));
    println!("{:?}", Counter(0).all());
    println!("{:?}", (1..3u8).all());
    println!("{:?}", vec![String::from("s")].into_iter().all());
    let direct: Vec<u32> = Counter(0).collect();
    println!("{direct:?}");
}
