// `as_deref()` of an `Option` or a `Result`: what's in it, dereferenced,
// `Option<String>` to `Option<&str>`.
use std::ops::Deref;
use std::rc::Rc;

struct Name(String);

impl Deref for Name {
    type Target = str;
    fn deref(&self) -> &str {
        println!("deref {}", self.0);
        &self.0
    }
}

fn greet(name: Option<&str>) -> String {
    match name {
        Some("root") => "hi admin".to_string(),
        Some(n) => format!("hi {n}"),
        None => "hi stranger".to_string(),
    }
}

fn main() {
    let owned: Option<String> = Some("ann".to_string());
    let none: Option<String> = None;
    println!("{} {}", greet(owned.as_deref()), greet(none.as_deref()));
    println!("{:?}", owned.as_deref().map(str::len));

    let items: Option<Vec<i32>> = Some(vec![3, 1, 2]);
    let slice: Option<&[i32]> = items.as_deref();
    println!("{:?} {:?}", slice.map(|s| s.len()), slice.and_then(|s| s.first()));

    let boxed: Option<Box<u8>> = Some(Box::new(7));
    println!("{:?}", boxed.as_deref().copied());
    let shared: Option<Rc<String>> = Some(Rc::new("rc".to_string()));
    println!("{:?}", shared.as_deref().map(|s| s.to_uppercase()));
    let named = Some(Name("custom".to_string()));
    println!("{:?}", named.as_deref());

    let ok: Result<String, i32> = Ok("fine".to_string());
    let err: Result<String, i32> = Err(4);
    println!("{:?} {:?}", ok.as_deref(), err.as_deref());
    let shared_ok: Result<Rc<String>, u8> = Ok(Rc::new("rc ok".to_string()));
    println!("{:?}", shared_ok.as_deref());
    if let Ok("fine") = ok.as_deref() {
        println!("matched");
    }
}
