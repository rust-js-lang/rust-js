// Generic code's `zip` of a range, a type parameter's value given no
// dictionary of its own, steps it as a JS iterator, which reads nothing by
// index: std's compiled `Zip` is told it isn't `TrustedRandomAccess`, the
// form's answer, never the range type's, whose index path would read what
// isn't there, `NaN`. It showed where the crate collects a zip by its fold,
// as a generic `extend` of a `take_while` makes it do (#2434).
struct Count(u32);

impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 <= 3).then_some(self.0)
    }
}

fn pairs<I: Iterator<Item = u32>, J: IntoIterator<Item = u32>>(it: I, arg: J) -> Vec<(u32, u32)> {
    it.zip(arg).collect()
}

fn extended<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    let mut v = vec![0];
    v.extend(it);
    v
}

fn main() {
    println!("{:?}", pairs(1..4u32, vec![1u32, 2, 3]));
    println!("{:?}", extended(Count(0).take_while(|_| true)));
}
