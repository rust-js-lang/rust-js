// A `map(|x| x)` over a `filter()` is std's `Map`, whose `count` is its
// fold: it asks no hint. The filter's own `count`, `Filter::count`, asks
// its source's hint first, which here runs the crate's `size_hint`. So the
// map is kept where generic code counts it, as it would count the filter
// itself otherwise (#2210).

/// Counts up to `.1` from `.0`, with its own `size_hint`.
struct Own(u32, u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        println!("own next");
        if self.0 == self.1 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        println!("own size_hint {n}");
        (n, Some(n))
    }
}

fn counted<I: Iterator>(it: I) -> usize {
    it.count()
}

fn even(x: &u32) -> bool {
    x % 2 == 0
}

fn main() {
    let chained = vec![7].into_iter().chain(Own(0, 3));
    println!("chained: {}", counted(chained.filter(even).map(|x| x)));

    let zipped = vec![7].into_iter().zip(Own(0, 3));
    println!("zipped: {}", counted(zipped.filter(|(x, _)| even(x)).map(|pair| pair)));

    let mut flattened = Some(Own(0, 3)).into_iter().flatten();
    flattened.next();
    println!("flattened: {}", counted(flattened.filter(even).map(|x| x)));

    let chained = vec![7].into_iter().chain(Own(0, 3));
    println!("mapped twice: {}", counted(chained.filter(even).map(|x| x).map(|x| x)));

    let mut filtered = vec![7].into_iter().chain(Own(0, 3)).filter(even);
    println!("lent: {}", counted(filtered.by_ref().map(|x| x)));

    let chained = vec![7].into_iter().chain(Own(0, 3));
    println!("unmapped: {}", counted(chained.filter(even)));

    let chained = vec![7].into_iter().chain(Own(0, 3));
    println!("here: {}", chained.filter(even).map(|x| x).count());
}
