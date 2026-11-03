// A tuple `match`'s pattern of a tuple in one of its items, `(true,
// Some((module, files)))`, binds its parts with one pattern, `const
// [module, files] = tests;`, as `if let` and a `match` of the item alone
// do: not `const module = tests[0]; const files = tests[1];`, as the
// playground's `prepare` picks its tests.
fn tuple(test: bool, tests: Option<(&str, &[u32])>, sources: &mut Vec<String>) -> Vec<u32> {
    match (test, tests) {
        (true, Some((module, files))) => {
            sources.push(module.to_string());
            files.to_vec()
        }
        _ => Vec::new(),
    }
}

fn alone(tests: Option<(&str, &[u32])>, sources: &mut Vec<String>) -> Vec<u32> {
    match tests {
        Some((module, files)) => {
            sources.push(module.to_string());
            files.to_vec()
        }
        None => Vec::new(),
    }
}

fn main() {
    let mut sources = Vec::new();
    let files = [1, 2];
    for test in [true, false] {
        for tests in [Some(("a", &files[..])), None] {
            println!(
                "{:?} {:?}",
                tuple(test, tests, &mut sources),
                alone(tests, &mut sources)
            );
        }
    }
    println!("{sources:?}");
}
