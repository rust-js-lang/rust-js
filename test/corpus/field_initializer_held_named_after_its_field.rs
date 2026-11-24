// A struct literal's field initializer held before a later field runs is
// named after the field it initializes, as hand-written JS names it and
// rust-analyzer suggests a value by its field: `const second =
// side("s", 2)`, where `side`, its call's name, would shadow the function
// it calls, and it was `value` (#1865). Its literal is `{ second, .. }`.
// `label("a")` held for `label` stays `value`: the field's word is the
// function's too, which it would shadow.
#[derive(Debug)]
struct Q {
    first: u32,
    second: u32,
}

#[derive(Debug)]
struct Labelled {
    label: String,
    count: usize,
}

fn side(what: &str, n: u32) -> u32 {
    println!("{what}");
    n
}

fn label(of: &str) -> String {
    format!("<{of}>")
}

fn main() {
    let mut base = Q { first: 1, second: 2 };
    let q = Q {
        second: side("s", 2),
        first: {
            base.first = 9;
            base.first
        },
    };
    let mut seen = Vec::new();
    let labelled = Labelled {
        label: label("a"),
        count: {
            seen.push(1);
            seen.len()
        },
    };
    println!("{q:?} {labelled:?} {} {seen:?}", base.second);
}
