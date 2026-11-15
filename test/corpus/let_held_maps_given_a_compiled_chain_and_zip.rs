struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn plus(g: &D, x: u32) -> u32 {
    g.0 + x
}
fn main() {
    let v = [1, 2, 3];
    let (ga, gb) = (D(5), D(6));
    let a = v.iter().map(move |x| plus(&ga, *x));
    let b = v.iter().map(move |x| plus(&gb, *x));
    let sum = a.chain(b).fold(0, |s, x| s + x);
    println!("chain {sum}");
    let ga = D(7);
    let a = v.iter().map(move |x| plus(&ga, *x));
    let sum = a.zip(0..10u32).fold(0, |s, (x, y)| s + x * y);
    println!("zip {sum}");

    // One whose hint is asked, held, std's struct, given a map made where
    // it's given.
    let (ga, gb) = (D(8), D(9));
    let a = v.iter().map(move |x| plus(&ga, *x));
    println!("hint {:?}", a.size_hint());
    let sum = a.zip(v.iter().map(move |x| plus(&gb, *x))).fold(0, |s, (x, y)| s + x * y);
    println!("zip {sum}");
    let (ga, gb) = (D(10), D(11));
    let mut a = v.iter().map(move |x| plus(&ga, *x));
    println!("hint {:?} {:?}", a.size_hint(), a.next());
    let sum = a.chain(v.iter().map(move |x| plus(&gb, *x))).fold(0, |s, x| s + x);
    println!("chain {sum}");
    let gb = D(12);
    let mut a = v.iter().enumerate();
    println!("hint {:?} {:?}", a.size_hint(), a.next_back());
    let b = [16, 17].iter().map(move |x| plus(&gb, *x));
    let pairs: Vec<_> = a.zip(b).collect();
    println!("collect {pairs:?}");
}
