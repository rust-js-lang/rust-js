// `size_hint()` of a `dyn Iterator`, boxed or lent, in a crate that asks
// one's: each value made one is made knowing its hint where it's made, so
// the JS iterator a `dyn Iterator` is gives std's, its value's (#1476).
struct Countdown(u32);
impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}
struct Sized3(u32);
impl Iterator for Sized3 {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}
fn hint(it: &mut dyn Iterator<Item = u32>) -> (usize, Option<usize>) {
    it.size_hint()
}
fn main() {
    boxed_hints();
    let v = vec![1u32, 2, 3, 4, 5, 6];
    let mut all: Vec<Box<dyn Iterator<Item = u32> + '_>> = vec![
        Box::new(Countdown(3)),
        Box::new(Sized3(4)),
        Box::new(v.iter().copied().chain(7..9)),
        Box::new(v.iter().copied().take(4)),
        Box::new(v.iter().copied().rev()),
        Box::new(v.iter().copied().enumerate().map(|(i, x)| i as u32 + x)),
        Box::new(v.iter().copied().zip(10..13).map(|(a, b)| a + b)),
        Box::new(v.iter().flat_map(|&x| vec![x; 2])),
        Box::new(0..5u32),
        Box::new([1u32, 2].into_iter().cycle().take(5)),
    ];
    for it in all.iter_mut() {
        let before = it.size_hint();
        it.next();
        println!("{:?} {:?} {:?}", before, it.size_hint(), hint(it));
    }
    let inner: Box<dyn Iterator<Item = u32>> = Box::new(v.iter().copied().skip(2));
    let outer: Box<dyn Iterator<Item = u32>> = Box::new(inner);
    println!("{:?}", outer.size_hint());
    let mut local = v.iter().copied().filter(|x| x % 2 == 1);
    println!("{:?}", hint(&mut local));
}

fn boxed<'a>(v: &'a [u32], even: bool) -> Box<dyn Iterator<Item = &'a u32> + 'a> {
    if even {
        Box::new(v.iter().filter(|x| *x % 2 == 0))
    } else {
        Box::new(v.iter().skip(1))
    }
}

fn boxed_hints() {
    let v = vec![1u32, 2, 3, 4, 5];
    let all: Box<dyn Iterator<Item = &u32>> = Box::new(v.iter());
    println!("{:?}", all.size_hint());
    let mut mapped: Box<dyn Iterator<Item = u32>> = Box::new(v.iter().map(|x| x * 2));
    mapped.next();
    println!("{:?}", mapped.size_hint());
    for even in [true, false] {
        let mut it = boxed(&v, even);
        println!("{:?}", it.size_hint());
        it.next();
        println!("{:?} {:?}", it.size_hint(), it.collect::<Vec<_>>());
    }
    let ranged: Box<dyn Iterator<Item = u32>> = Box::new((0..10).step_by(3));
    println!("{:?}", ranged.size_hint());
}
