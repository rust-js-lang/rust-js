// A value `if` given as an argument in a loop over a `RefCell`'s borrow,
// one `try` around the loop, is the conditional it is without the borrow
// (#413): React Float's hydrateHoistable, `node.getAttribute("href") !==
// (href == null || href === "" ? null : href)`, not a `let` of it set in
// an `if` and its `else`. And a condition written with `||` of such
// values, each side's jump the one `continue`, one `if` of it all,
// `if (a !== (..) || b !== (..)) { continue; }`, not an `if` of each side
// (rd225); two `if`s written apart stay two.
use std::cell::RefCell;

fn attribute(node: &str) -> Option<&str> {
    node.strip_prefix("href=")
}

fn blank(href: Option<&str>) -> bool {
    href.is_some_and(str::is_empty)
}

fn find(cache: &RefCell<Vec<&'static str>>, href: Option<&str>) -> Option<usize> {
    let mut nodes = cache.borrow_mut();
    for i in 0..nodes.len() {
        let node = nodes[i];
        if attribute(node) != if href.is_none() || blank(href) { None } else { href } {
            continue;
        }
        nodes.remove(i);
        return Some(i);
    }
    None
}

fn matched(
    cache: &RefCell<Vec<(&'static str, &'static str)>>,
    rel: Option<&str>,
    title: Option<&str>,
) -> Option<usize> {
    let nodes = cache.borrow();
    for (i, &(r, t)) in nodes.iter().enumerate() {
        if Some(r) != if rel.is_none() { None } else { rel }
            || Some(t) != if title.is_none() { None } else { title }
            || blank(Some(t))
        {
            continue;
        }
        return Some(i);
    }
    None
}

fn apart(nodes: &[(&str, &str)], rel: &str, title: &str) -> Option<usize> {
    for (i, &(r, t)) in nodes.iter().enumerate() {
        if r != rel {
            continue;
        }
        if t != title {
            continue;
        }
        return Some(i);
    }
    None
}

fn main() {
    let pairs = RefCell::new(vec![("a", "x"), ("b", ""), ("b", "y")]);
    for (rel, title) in [
        (Some("b"), Some("y")),
        (Some("a"), Some("x")),
        (Some("b"), Some("")),
        (None, None),
    ] {
        println!("{:?}", matched(&pairs, rel, title));
    }
    println!(
        "{:?} {:?}",
        apart(&pairs.borrow(), "b", "y"),
        apart(&pairs.borrow(), "c", "y")
    );
    let cache = RefCell::new(vec!["rel", "href=a", "href=b", "href="]);
    for href in [Some("b"), None, Some(""), Some("a"), Some("z")] {
        println!("{href:?} {:?} {:?}", find(&cache, href), cache.borrow());
    }
}
