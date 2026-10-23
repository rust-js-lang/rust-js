// `rposition` of a lazy iterator that's exact-sized and double-ended, made
// knowing its back and its length as `as_exact_size` makes it: a map or an
// `enumerate` of an array's items made in place, a copy, a `rev`, a
// string's bytes, and the crate's own.

struct C(u32);
impl Iterator for C {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 5 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = (5 - self.0) as usize;
        (left, Some(left))
    }
}
impl DoubleEndedIterator for C {
    fn next_back(&mut self) -> Option<u32> {
        if self.0 < 5 {
            let v = 5;
            self.0 += 1;
            Some(v + self.0 - 1)
        } else {
            None
        }
    }
}
impl ExactSizeIterator for C {}
fn main() {
    let v = vec![1, 2, 3, 2, 5];
    println!("{:?}", v.iter().map(|x| x * 2).rposition(|x| x == 4));
    println!("{:?}", v.iter().copied().map(|x| x + 1).rposition(|x| x == 3));
    println!("{:?}", v.iter().rev().rposition(|&x| x == 2));
    println!("{:?}", "héllo".bytes().rposition(|b| b == b'l'));
    println!("{:?}", v.iter().enumerate().rposition(|(i, &x)| i > 0 && x == 2));
    println!("{:?}", C(0).rposition(|x| x == 7));
}
