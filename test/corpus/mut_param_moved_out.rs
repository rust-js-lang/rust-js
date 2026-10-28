// A `mut` parameter is its argument first: what's moved out of it, by a
// pattern or into a `move` closure, keeps that value when the parameter
// is assigned again, as a `let mut`'s does; and so does a read of it an
// operand or a tuple takes before an assignment after it.
struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

fn renamed(mut name: Option<String>) -> String {
    match name {
        Some(old) => {
            name = Some("new".into());
            format!("{old} {name:?}")
        }
        None => String::new(),
    }
}

fn reverse(mut head: Option<Box<Node>>) -> Option<Box<Node>> {
    let mut reversed = None;
    while let Some(mut node) = head {
        head = node.next.take();
        node.next = reversed;
        reversed = Some(node);
    }
    reversed
}

fn scaled(mut factor: i32) -> i32 {
    let scale = move |x: i32| x * factor;
    factor = 5;
    scale(2) + factor
}

fn counted(mut n: u32) -> u32 {
    n + {
        n += 1;
        n
    }
}

fn paired(mut name: String) -> (String, String) {
    (name.clone(), {
        name = "new".into();
        name
    })
}

fn main() {
    println!("{}", renamed(Some("old".into())));
    let list = Some(Box::new(Node {
        value: 1,
        next: Some(Box::new(Node { value: 2, next: None })),
    }));
    let mut node = reverse(list);
    while let Some(n) = node {
        print!("{} ", n.value);
        node = n.next;
    }
    println!();
    println!("{}", scaled(3));
    println!("{} {:?}", counted(1), paired("old".into()));
    let shifted = |mut by: i32| {
        let at = move || by;
        by += 10;
        at() * 100 + by
    };
    println!("{}", shifted(1));
}
