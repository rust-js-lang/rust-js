//@ library-differs
// The crate's own iterator given to generic code that gives it back behind
// an `impl Iterator`, as it is, wrapped in the crate's struct, or mapped:
// the caller reveals the hidden type as the value it gave (#1142), so the
// type parameter is given as it is, its own value stepped by its `next`, as
// one given back by name is (#1230). Through two such, in a method of its
// own, and of an `into_iter()` of items in generic code.
struct Count(u32);

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 < 3 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
}

impl Count {
    fn doubled(self) -> Vec<u32> {
        hidden(hidden(self.into_iter()).map(|n| n * 2)).collect()
    }
}

struct Tagged<I>(I, &'static str);

impl<I: Iterator<Item = u32>> Iterator for Tagged<I> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        self.0.next().map(|n| format!("{}{n}", self.1))
    }
}

fn same<I: Iterator>(it: I) -> I {
    it
}

fn hidden<I: Iterator>(it: I) -> impl Iterator<Item = I::Item> {
    it
}

fn tagged<I: Iterator<Item = u32>>(it: I, tag: &'static str) -> impl Iterator<Item = String> {
    Tagged(it, tag)
}

fn squared<I: Iterator<Item = u32>>(it: I) -> impl Iterator<Item = u32> {
    it.map(|n| n * n)
}

fn sums<I: Iterator>(items: I) -> Vec<i32>
where
    I::Item: IntoIterator<Item = i32>,
{
    items.map(|item| hidden(item.into_iter()).sum()).collect()
}

fn main() {
    let named: Vec<u32> = same(Count(0)).map(|n| n * 2).collect();
    let mapped: Vec<u32> = hidden(Count(0)).map(|n| n * 2).collect();
    let whole: Vec<u32> = hidden(Count(0).into_iter()).collect();
    println!("{named:?} {mapped:?} {whole:?}");
    println!("{:?}", Count(0).doubled());
    println!("{:?}", hidden(hidden(Count(1))).collect::<Vec<_>>());
    println!("{:?}", tagged(Count(0), "n").collect::<Vec<_>>());
    println!("{:?}", tagged(hidden(Count(1)), "m").last());
    println!("{:?}", squared(Count(0)).sum::<u32>());
    let mut stepped = hidden(Count(0));
    println!("{:?} {:?}", stepped.next(), stepped.collect::<Vec<_>>());
    println!("{:?}", sums(vec![Some(1), None, Some(3)].into_iter()));
    println!("{:?}", sums(vec![vec![1, 2], vec![3]].into_iter()));
}
