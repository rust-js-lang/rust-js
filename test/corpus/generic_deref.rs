// A `P: Deref` in generic code is given a dictionary of its `deref` (ADR
// 0394): std's, the value itself of a reference, a `Box`, an `Rc` or text,
// as each is its value; the crate's own, its impl's. A `&mut ()` is
// nothing, as writing a `()` is.
use std::ops::Deref;
use std::rc::Rc;

struct Tracked {
    name: String,
}

impl Deref for Tracked {
    type Target = String;
    fn deref(&self) -> &String {
        println!("deref {}", self.name);
        &self.name
    }
}

fn shout<P: Deref<Target = str>>(p: P) -> String {
    p.to_uppercase()
}

fn length<P>(p: &P) -> usize
where
    P: Deref,
    P::Target: AsRef<str>,
{
    (**p).as_ref().len()
}

struct Wrapper<P>(P);

impl<P: Deref<Target = i32>> Wrapper<P> {
    fn doubled(&self) -> i32 {
        *self.0 * 2
    }
}

fn main() {
    println!("{}", shout(Box::<str>::from("box")));
    println!("{}", shout(String::from("string")));
    println!("{}", length(&Rc::new(String::from("rc"))));
    println!("{}", length(&Tracked { name: "mine".to_string() }));
    let n = 21;
    println!("{} {}", Wrapper(&n).doubled(), Wrapper(Box::new(4)).doubled());
    let mut unit = ();
    let held = Wrapper(&mut unit);
    let Wrapper(()) = Wrapper(*held.0);
}
