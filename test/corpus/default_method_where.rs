//@ library-differs
// A trait's default method whose `where` clauses an impl's type can't meet,
// `where Self::Item: Deref`, isn't in that impl's dictionary, as rustc
// leaves it out of the vtable: nothing can call it. An impl that meets them
// has it, through a `dyn` or generic code alike.
use std::ops::Deref;

trait Source {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;

    fn first_cloned(mut self) -> Option<<Self::Item as Deref>::Target>
    where
        Self: Sized,
        Self::Item: Deref,
        <Self::Item as Deref>::Target: Clone,
    {
        self.next().map(|x| x.deref().clone())
    }

    fn label(&self) -> String {
        "source".to_string()
    }
}

struct Counter {
    value: i32,
}

struct Token {
    value: i32,
}

impl Source for Counter {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        let x = self.value;
        self.value += 1;
        Some(Token { value: x })
    }
}

struct Names {
    names: Vec<Box<String>>,
}

impl Source for Names {
    type Item = Box<String>;

    fn next(&mut self) -> Option<Box<String>> {
        self.names.pop()
    }

    fn label(&self) -> String {
        format!("{} names", self.names.len())
    }
}

fn label_of<S: Source>(s: &S) -> String {
    s.label()
}

fn main() {
    let mut boxed: Box<dyn Source<Item = Token>> = Box::new(Counter { value: 22 });
    println!("{} {}", boxed.next().unwrap().value, boxed.label());
    println!("{}", label_of(&Counter { value: 0 }));
    let names = Names { names: vec![Box::new("a".to_string()), Box::new("b".to_string())] };
    println!("{}", label_of(&names));
    println!("{:?}", names.first_cloned());
    let mut dyn_names: Box<dyn Source<Item = Box<String>>> = Box::new(Names { names: vec![] });
    println!("{:?} {}", dyn_names.next(), dyn_names.label());
}
