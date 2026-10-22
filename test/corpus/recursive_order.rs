// A derived order of a type inside itself: a function that calls itself
// for the ones inside, as its clone is.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Tree {
    Leaf(i32),
    Node(Box<Tree>, Box<Tree>),
}

#[derive(PartialEq, PartialOrd)]
enum Num {
    Float(f64),
    Neg(Box<Num>),
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct List {
    head: u8,
    next: Option<Box<List>>,
}

fn leaf(n: i32) -> Box<Tree> {
    Box::new(Tree::Leaf(n))
}

fn main() {
    let mut trees = vec![
        Tree::Node(leaf(2), leaf(1)),
        Tree::Leaf(5),
        Tree::Node(leaf(1), Box::new(Tree::Node(leaf(0), leaf(0)))),
        Tree::Node(leaf(1), leaf(9)),
    ];
    trees.sort();
    println!("{trees:?}");
    println!("{:?}", trees.iter().max());

    let nan = Num::Neg(Box::new(Num::Float(f64::NAN)));
    let one = Num::Neg(Box::new(Num::Float(1.0)));
    println!("{:?} {}", nan.partial_cmp(&one), one < Num::Neg(Box::new(Num::Float(2.0))));

    let short = List { head: 1, next: None };
    let long = List { head: 1, next: Some(Box::new(List { head: 0, next: None })) };
    println!("{:?}", short.cmp(&long));
}
