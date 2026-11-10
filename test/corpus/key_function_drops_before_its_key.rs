// `max_by_key` and `min_by_key` whose key has a destructor and whose
// function holds a value with one: std's `let (_, x) =
// self.map(key(f)).max_by(compare)?;` drops the `Map` holding `f` as
// `max_by` returns, then the kept pair's key as the `let` takes it apart:
// the function before the key it kept (#1231).
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Key(i32);

impl Drop for Key {
    fn drop(&mut self) {
        println!("drop key {}", self.0);
    }
}

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let held = Noisy("max's closure");
    let dropping = move |x: &i32| {
        let _ = &held;
        Key(x % 7)
    };
    println!("{:?}", [5, 9, 13, 2].into_iter().max_by_key(dropping));
    let held = Noisy("min's closure");
    let dropping = move |x: &i32| {
        let _ = &held;
        Key(x % 7)
    };
    println!("{:?}", [5, 9, 13, 2].into_iter().min_by_key(dropping));
    let held = Noisy("boxed closure");
    let boxed: Box<dyn FnMut(&i32) -> Key> = Box::new(move |x: &i32| {
        let _ = &held;
        Key(x % 5)
    });
    println!("{:?}", [5, 9, 13, 2].into_iter().max_by_key(boxed));
    println!("{:?}", Vec::<i32>::new().into_iter().max_by_key(|x| Key(*x)));
}
