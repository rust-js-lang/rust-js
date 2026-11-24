//@ library-differs
// #2055 (ADR 0861, amended): a `pub` function JS or a consumer calls gives
// its `None` back as `undefined`, Rust's, at its exit (`returns_one_none`),
// though each of its returns is `null` or a value, a nullable function's
// `parent_of(node)` or `Some(node)`. So it isn't found to return `null` or a
// value (rd632), and what the crate's own call gives a nullable parameter
// is tested `!= null`, either; as a library, `dispatch(event_target(&lone))`
// read `.id` of `undefined`. Natively, the same answers.
pub struct Node {
    pub id: u32,
    pub parent: Option<Box<Node>>,
    pub text: bool,
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn parent_of(node: &Node) -> Option<&Node> {
    node.parent.as_deref()
}

pub fn event_target(node: &Node) -> Option<&Node> {
    if node.text { parent_of(node) } else { Some(node) }
}

#[cfg_attr(rust_js, rust_js::nullable(target))]
fn dispatch(target: Option<&Node>) -> u32 {
    match target {
        Some(target) => target.id,
        None => 0,
    }
}

pub fn run(node: &Node) -> u32 {
    dispatch(event_target(node))
}

fn main() {
    let root = Node {
        id: 3,
        parent: None,
        text: false,
    };
    let text = Node {
        id: 1,
        parent: Some(Box::new(root)),
        text: true,
    };
    let lone = Node {
        id: 5,
        parent: None,
        text: true,
    };
    println!("{} {} {}", run(&text), run(&lone), run(text.parent.as_deref().unwrap()));
    println!("{:?}", event_target(&lone).map(|node| node.id));
}
