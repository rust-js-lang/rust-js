// `Vec::leak` and `Box::leak` keep a value for good, a `&'static` of it:
// in JS, which frees nothing itself, the value itself, as react.dev's
// Challenges keeps the challenges its effect and its render share.
#[derive(Debug)]
struct Titles {
    first: &'static str,
    count: usize,
}

fn titles() -> &'static [&'static str] {
    Vec::leak(vec!["Note", "Pitfall"])
}

fn kept() -> &'static Titles {
    Box::leak(Box::new(Titles { first: "Note", count: 2 }))
}

fn main() {
    let shared = titles();
    let also = shared;
    println!("{:?} {} {}", shared, also.len(), also[1]);
    let titles = kept();
    println!("{:?} {}", titles, titles.first.len() + titles.count);
}
