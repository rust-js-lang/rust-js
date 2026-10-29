// A value `if` given as an argument in a loop over a `RefCell`'s borrow,
// one `try` around the loop, is the conditional it is without the borrow
// (#413): React Float's hydrateHoistable, `node.getAttribute("href") !==
// (href == null || href === "" ? null : href)`, not a `let` of it set in
// an `if` and its `else`.
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

fn main() {
    let cache = RefCell::new(vec!["rel", "href=a", "href=b", "href="]);
    for href in [Some("b"), None, Some(""), Some("a"), Some("z")] {
        println!("{href:?} {:?} {:?}", find(&cache, href), cache.borrow());
    }
}
