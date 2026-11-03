// rd632 (ADR 0861, amended): what's given a function's own nullable
// parameter is `null` or a value, so its body tests it `=== null`, where
// it's a crate function's result whose every return is so (React DOM's
// `getEventTarget`, `target.nodeType === TEXT_NODE ? target.parentNode :
// target`), a temporary each of whose writes is so (`if profiling {
// reason() } else { no_reason() }`), or a shared `as_deref()` of such
// (commitPlacement's `parentFragmentInstances.as_deref()`). A function
// that may return a `None` it makes, `undefined`, or its own nullable
// parameter, and a temporary one of whose writes is a `None`, keep the
// test `== null`, as JS gives them `undefined`. Natively, the same answers.
pub struct Node {
    pub id: u32,
    pub parent: Option<Box<Node>>,
    pub text: bool,
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn parent_of(node: &Node) -> Option<&Node> {
    node.parent.as_deref()
}

fn maybe_target(node: &Node) -> Option<&Node> {
    if node.text { None } else { Some(node) }
}

#[cfg_attr(rust_js, rust_js::nullable(target))]
fn passed_on(target: Option<&Node>) -> Option<&Node> {
    target
}

fn given_back(node: &Node) -> Option<&Node> {
    passed_on(maybe_target(node))
}

#[cfg_attr(rust_js, rust_js::nullable(target))]
fn dispatch(target: Option<&Node>) -> u32 {
    match target {
        Some(target) => target.id,
        None => 0,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(target))]
fn dispatch_loose(target: Option<&Node>) -> u32 {
    match target {
        Some(target) => target.id,
        None => 0,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(target))]
fn dispatch_given_back(target: Option<&Node>) -> u32 {
    match target {
        Some(target) => target.id,
        None => 0,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(reason))]
fn commit(reason: Option<&str>) -> usize {
    match reason {
        Some(reason) => reason.len(),
        None => 0,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(reason))]
fn commit_loose(reason: Option<&str>) -> usize {
    match reason {
        Some(reason) => reason.len(),
        None => 0,
    }
}

#[cfg_attr(rust_js, rust_js::nullable(reason))]
fn commit_reassigned(reason: Option<&str>) -> usize {
    match reason {
        Some(reason) => reason.len(),
        None => 0,
    }
}

// A parameter written once holds what its caller gave too.
pub fn reassigned(mut reason: Option<&'static str>, set: bool) -> usize {
    if set {
        reason = Some("set");
    }
    commit_reassigned(reason)
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn suspended_reason(node: &Node) -> Option<&'static str> {
    if node.id > 1 { Some("Throttled") } else { None }
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn no_reason() -> Option<&'static str> {
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn instances_of(node: &Node) -> Option<Vec<u32>> {
    let mut instances = None;
    let mut parent = node.parent.as_deref();
    while let Some(current) = parent {
        match &mut instances {
            None => instances = Some(vec![current.id]),
            Some(found) => found.push(current.id),
        }
        parent = current.parent.as_deref();
    }
    instances
}

#[cfg_attr(rust_js, rust_js::nullable(instances))]
fn placed(instances: Option<&[u32]>) -> u32 {
    let Some(instances) = instances else {
        return 0;
    };
    instances.iter().sum()
}

pub fn run(node: &Node, profiling: bool) -> (u32, u32, u32, usize, usize, u32) {
    let target = event_target(node);
    let reason = commit(if profiling { suspended_reason(node) } else { no_reason() });
    let loose = commit_loose(if profiling { suspended_reason(node) } else { None });
    let instances = instances_of(node);
    (
        dispatch(target),
        dispatch_loose(maybe_target(node)),
        dispatch_given_back(given_back(node)),
        reason,
        loose,
        placed(instances.as_deref()),
    )
}

#[cfg_attr(rust_js, rust_js::nullable(target))]
fn dispatch_direct(target: Option<&Node>) -> u32 {
    match target {
        Some(target) => target.id,
        None => 0,
    }
}

// Its call given straight to the parameter, no `let` that's `null` for
// `None` where it's written: what its first lowering saw of the call, a
// value that may be `undefined`, is forgotten once `event_target` is found.
pub fn direct(node: &Node) -> u32 {
    dispatch_direct(event_target(node))
}

// After `run` and `direct`, which are lowered first, then again once this
// is found to return `null` or a value.
fn event_target(node: &Node) -> Option<&Node> {
    if node.text { parent_of(node) } else { Some(node) }
}

fn main() {
    let root = Node {
        id: 3,
        parent: None,
        text: false,
    };
    let element = Node {
        id: 2,
        parent: Some(Box::new(root)),
        text: false,
    };
    let text = Node {
        id: 1,
        parent: Some(Box::new(element)),
        text: true,
    };
    println!("{:?}", run(&text, true));
    println!("{:?}", run(&text, false));
    let lone = Node {
        id: 5,
        parent: None,
        text: true,
    };
    println!("{:?}", run(&lone, true));
    let element = text.parent.as_deref().unwrap();
    println!("{:?}", run(element, true));
    println!("{} {} {}", direct(&text), direct(&lone), direct(element));
    println!("{} {} {}", reassigned(None, false), reassigned(None, true), reassigned(Some("given"), false));
}
