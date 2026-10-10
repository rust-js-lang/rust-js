// A leaked `String`'s `&mut str` is a box of it, as any `&mut str` is
// (ADR 0334): written through in a variable, given to a function and back,
// kept in a `Vec` and made by `map(String::leak)`. `Box::leak` of a
// number gives one too, a leak read as a `&'static str` is the string, and
// what's leaked is never dropped.
fn keep(s: &'static mut str) -> &'static mut str {
    s
}

#[derive(Debug)]
struct Noisy;

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("dropped");
    }
}

fn bump(n: &mut i32) {
    *n += 1;
}

fn main() {
    let shout = format!("ab").leak();
    shout.make_ascii_uppercase();
    println!("{shout}");
    let other = keep(String::from("cd").leak());
    other.make_ascii_uppercase();
    println!("{other}");
    let mut kept: Vec<&'static mut str> = vec![String::from("ef").leak()];
    kept[0].make_ascii_uppercase();
    kept.push(String::from("gh").leak());
    println!("{kept:?}");
    let read: &'static str = String::from("ij").leak();
    println!("{read}");
    let mapped: Vec<&'static mut str> = vec![String::from("kl")].into_iter().map(String::leak).collect();
    for text in mapped {
        text.make_ascii_uppercase();
        println!("{text}");
    }
    let noisy: &'static mut Noisy = Box::leak(Box::new(Noisy));
    println!("{noisy:?} kept");

    let r: &'static mut i32 = Box::leak(Box::new(5));
    *r += 1;
    bump(r);
    println!("{r}");
    let counts: Vec<&'static mut i32> = vec![Box::leak(Box::new(1))];
    for count in counts {
        *count += 1;
        println!("{count}");
    }
}
