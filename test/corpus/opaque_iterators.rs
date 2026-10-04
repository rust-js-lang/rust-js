// An `impl Iterator` a function returns is the iterator it stands for: a
// method taking it by `&mut`, as `next()`, `any` and `find` do, steps that
// one, not a box around it.

fn tens(v: &[i32]) -> impl Iterator<Item = i32> + '_ {
    v.iter().map(|&x| x * 10)
}

fn evens(n: u32) -> impl Iterator<Item = u32> {
    (0..n).filter(|x| x % 2 == 0)
}

fn main() {
    let v = vec![1, 2, 3];
    println!("{:?} {:?} {:?}", tens(&v).next(), tens(&v).nth(1), tens(&v).nth(5));
    println!("{} {}", tens(&v).any(|x| x > 25), tens(&v).all(|x| x > 5));
    println!("{:?} {:?}", tens(&v).find(|&x| x > 15), tens(&v).position(|x| x == 20));
    println!("{:?} {} {:?}", tens(&v).max(), tens(&v).count(), tens(&v).last());
    let mut kept = tens(&v);
    println!("{:?} {:?}", kept.next(), kept.next());
    println!("{:?} {:?}", evens(7).next(), evens(7).find(|x| *x > 2));
}
