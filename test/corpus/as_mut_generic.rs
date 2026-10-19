fn shout<T: AsMut<str>>(mut t: T) -> T {
    t.as_mut().make_ascii_uppercase();
    t
}

fn double<T: AsMut<[u32]>>(mut t: T) -> T {
    for x in t.as_mut() {
        *x *= 2;
    }
    t
}

fn main() {
    println!("{}", shout(String::from("hi")));
    println!("{:?}", double(vec![1, 2]));
    println!("{:?}", double([3, 4]));
    let mut s = String::from("ab");
    shout(&mut s);
    println!("{s}");
    let mut v = vec![5, 6];
    double(&mut v);
    println!("{v:?}");
}
