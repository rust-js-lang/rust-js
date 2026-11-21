// A held `rev()` of an adapter made in place over an array's items, a
// `skip(1)`, whose hint is asked: the `skip` made knowing its hint,
// `$skipping`, which steps from its back as std's `Skip` does (#2141), so
// its hint, back, length and folds from the back are std's (#1427).
#[derive(Debug, Clone)]
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn hint<I: Iterator>(it: &I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn copied_back() {
    let v = vec![1u8, 2, 3, 4];
    let mut it = v.iter().skip(1).copied().rev();
    println!("{:?}", it.size_hint());
    println!("{:?}", it.next());
    println!("{:?} {}", it.size_hint(), it.len());
    println!("{:?}", it.next_back());
    println!("{:?}", it.size_hint());
    println!("{:?}", it.collect::<Vec<_>>());

    let w = vec![1u8, 2, 3];
    let mut it = w.iter().skip(1).copied().rev();
    println!("{:?}", it.size_hint());
    println!("{}", it.by_ref().fold(0, |a, x| a + x));
    println!("{:?}", it.size_hint());

    let mut it = v.iter().skip(5).copied().rev();
    println!("{:?} {:?}", it.size_hint(), it.next());

    let l = vec![Loud(1), Loud(2), Loud(3)];
    let mut it = l.iter().skip(1).cloned().rev();
    println!("{:?}", it.size_hint());
    println!("{:?}", it.next().map(|x| x.0));
    println!("{:?}", it.size_hint());
    drop(it);

    let mut it = v.iter().skip(2).rev();
    println!("{:?} {:?} {:?}", it.size_hint(), it.next(), it.size_hint());
}

fn other_forms() {
    let v = vec![1u8, 2, 3, 4, 5, 6];
    let mut a = v.iter().skip(1).filter_map(|x| (x % 2 == 0).then_some(*x)).rev();
    println!("{:?} {:?} {:?}", a.size_hint(), a.next(), a.size_hint());
    let mut b = v.iter().skip(1).take(3).rev();
    println!("{:?} {:?} {:?} {}", b.size_hint(), b.next(), b.size_hint(), b.len());
    let mut c = v.iter().skip(1).step_by(2).rev();
    println!("{:?} {:?} {:?}", c.size_hint(), c.next(), c.size_hint());
    let mut d = v.iter().enumerate().skip(2).rev();
    println!("{:?} {:?} {:?}", d.size_hint(), d.next(), hint(&d));
    let mut e = v.iter().skip(1).copied().rev();
    println!("{:?} {:?} {:?}", e.nth_back(1), e.size_hint(), e.rposition(|x| x == 4));
    println!("{:?} {:?}", e.size_hint(), e.rfold(0u32, |a, x| a * 10 + x as u32));
    let mut f = v.iter().skip(1).map(|x| x * 2).rev();
    println!("{:?} {:?} {:?}", f.size_hint(), f.next(), f.size_hint());
    let l = vec![Loud(1), Loud(2), Loud(3), Loud(4)];
    let mut g = l.into_iter().skip(1).rev();
    println!("{:?}", g.size_hint());
    println!("{:?}", g.next().map(|x| x.0));
    println!("{:?}", g.size_hint());
    drop(g);
    let s = [10u16, 20, 30];
    let mut h = s.iter().skip(1).rev().skip(1);
    println!("{:?} {:?} {:?}", h.size_hint(), h.next(), h.size_hint());
}

fn main() {
    copied_back();
    other_forms();
    println!("end");
}
