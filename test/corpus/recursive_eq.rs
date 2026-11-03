// A derived `==` of a type inside itself with a part of its own `==`,
// regex-syntax's `Hir`, whose class is an `IntervalSet` compared by hand:
// a function that calls itself for the ones inside, as its clone and its
// order are. In place, its comparison of the one inside never ended.
#[derive(Debug, Clone)]
struct Set(Vec<u32>);

// Equal as sets, whatever their order.
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

#[derive(Debug, Clone, PartialEq)]
struct Hir {
    kind: HirKind,
    props: Box<Props>,
}

#[derive(Debug, Clone, PartialEq)]
struct Props {
    min: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum HirKind {
    Empty,
    Class(Set),
    Concat(Vec<Hir>),
    Capture { sub: Box<Hir>, name: Option<Box<str>> },
}

// One inside the other, each inside itself through the other.
#[derive(Debug, PartialEq)]
struct Group {
    set: Set,
    rule: Option<Box<Rule>>,
}

#[derive(Debug, PartialEq)]
struct Rule {
    groups: Vec<Group>,
}

fn class(items: &[u32]) -> Hir {
    Hir {
        kind: HirKind::Class(Set(items.to_vec())),
        props: Box::new(Props { min: 1 }),
    }
}

fn capture(sub: Hir, name: Option<&str>) -> Hir {
    let min = sub.props.min;
    Hir {
        kind: HirKind::Capture {
            sub: Box::new(sub),
            name: name.map(Box::from),
        },
        props: Box::new(Props { min }),
    }
}

fn main() {
    let a = capture(class(&[3, 1, 2]), Some("x"));
    let b = capture(class(&[1, 2, 3, 3]), Some("x"));
    let c = capture(class(&[1, 2]), Some("x"));
    let d = capture(class(&[1, 2, 3]), None);
    println!("{} {} {} {}", a == b, a == c, a != d, a == a.clone());

    let concat = |parts: Vec<Hir>| Hir {
        kind: HirKind::Concat(parts),
        props: Box::new(Props { min: 0 }),
    };
    let e = concat(vec![a.clone(), class(&[7])]);
    let f = concat(vec![b.clone(), class(&[7, 7])]);
    let g = concat(vec![b.clone()]);
    println!("{} {}", e == f, e == g);

    let empty = Hir {
        kind: HirKind::Empty,
        props: Box::new(Props { min: 0 }),
    };
    let all = [a.clone(), empty.clone(), e.clone()];
    println!("{} {} {}", all.contains(&b), all.contains(&empty), all.contains(&c));
    assert_eq!(Some(&f), all.last());

    let rule = |sets: &[&[u32]]| Rule {
        groups: sets
            .iter()
            .map(|s| Group {
                set: Set(s.to_vec()),
                rule: None,
            })
            .collect(),
    };
    let x = Group {
        set: Set(vec![1]),
        rule: Some(Box::new(rule(&[&[2, 3], &[4]]))),
    };
    let y = Group {
        set: Set(vec![1, 1]),
        rule: Some(Box::new(rule(&[&[3, 2], &[4]]))),
    };
    let z = Group {
        set: Set(vec![1]),
        rule: Some(Box::new(rule(&[&[2, 3]]))),
    };
    println!("{} {} {:?}", x == y, x == z, x.rule.as_deref() == y.rule.as_deref());
}
