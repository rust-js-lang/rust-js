// A type inside itself is compared, cloned and ordered by a function that
// calls itself for the ones inside (#511): one for each type and module,
// which every place of the module that compares, clones or orders one
// calls, as a person writes it (#517). Each module has its own, the
// functions it reads imported, a variable of the same name kept apart.
mod sets {
    // Equal as sets, whatever their order: a part of `Expr`'s `==` that
    // `$eq` can't compare.
    #[derive(Debug, Clone, PartialOrd, Ord)]
    pub struct Set(pub Vec<u32>);

    impl PartialEq for Set {
        fn eq(&self, other: &Set) -> bool {
            let mut a = self.0.clone();
            let mut b = other.0.clone();
            a.sort_unstable();
            b.sort_unstable();
            a.dedup();
            b.dedup();
            a == b
        }
    }

    impl Eq for Set {}
}

mod exprs {
    use super::sets::Set;

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Expr {
        Num(i64),
        Class(Set),
        Add(Box<Expr>, Box<Expr>),
        List(Vec<Expr>),
    }

    pub fn add(a: Expr, b: Expr) -> Expr {
        Expr::Add(Box::new(a), Box::new(b))
    }

    // Its `==`, clone and order where its module compares, clones and
    // orders one, each more than once.
    pub fn report(a: &Expr, b: &Expr) -> String {
        let copy = a.clone();
        let other = b.clone();
        format!(
            "{} {} {} {:?} {:?}",
            copy == *a,
            other == *a,
            a != b,
            a.cmp(b),
            other.partial_cmp(&copy)
        )
    }

    // A variable named as the module's function is, in a function lowered
    // after it was made, which mustn't hide it.
    pub fn twice(a: &Expr, b: &Expr) -> (bool, bool) {
        let eq_expr = a == b;
        (eq_expr, b.clone() == *a)
    }
}

// Of a type only a caller knows, the function of the function given its
// dictionaries.
#[derive(Debug, Clone, PartialEq)]
enum Tree<T> {
    Leaf(T),
    Node(Vec<Tree<T>>),
}

fn same<T: PartialEq + Clone>(a: &Tree<T>, b: &Tree<T>) -> bool {
    a.clone() == *b
}

use exprs::{Expr, add};
use sets::Set;

fn main() {
    let a = add(Expr::Class(Set(vec![3, 1, 2])), Expr::Num(1));
    let b = add(Expr::Class(Set(vec![1, 2, 3, 3])), Expr::Num(1));
    let c = add(Expr::Class(Set(vec![1, 2])), Expr::List(vec![Expr::Num(2)]));
    println!("{}", exprs::report(&a, &b));
    println!("{}", exprs::report(&a, &c));

    // A variable named as the function is, which mustn't hide it.
    let eq_expr = a == b;
    let list = Expr::List(vec![a.clone(), c.clone()]);
    let other = Expr::List(vec![b.clone(), c.clone()]);
    println!("{} {} {}", eq_expr, list == other, list == Expr::List(vec![c.clone()]));

    let mut all = vec![c.clone(), list.clone(), b.clone(), a.clone(), Expr::Num(-4)];
    all.sort();
    all.dedup();
    println!("{:?}", all);
    println!("{} {:?}", all.contains(&other), list.cmp(&other));
    println!("{:?}", all.iter().max());
    println!("{:?} {:?}", exprs::twice(&a, &b), exprs::twice(&a, &c));

    let t = Tree::Node(vec![Tree::Leaf(Set(vec![1, 2])), Tree::Node(vec![])]);
    let u = Tree::Node(vec![Tree::Leaf(Set(vec![2, 1, 1])), Tree::Node(vec![])]);
    let v = Tree::Node(vec![Tree::Leaf(Set(vec![2])), Tree::Node(vec![])]);
    println!("{} {}", same(&t, &u), same(&t, &v));
}
