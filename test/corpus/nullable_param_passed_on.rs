// rd624 (P1, ADR 0861, amended): a function's own nullable parameter
// passed on to another's is `null` or a value only where it is itself,
// each passing on followed to its end. React's reconciler's public
// `updateContainer` takes plain `Option`s, which a call may leave
// `undefined`, and gives them to `updateContainerImpl`, whose nullable
// `parentComponent` goes on to `getContextForSubtree`'s: that one is
// tested `!= null`, not `=== null`, so a `None` left out is the empty
// context, against native Rust.
pub struct Component {
    pub context: u32,
}

#[cfg_attr(rust_js, rust_js::nullable(parent_component))]
fn get_context_for_subtree(parent_component: Option<&Component>) -> u32 {
    let Some(parent_component) = parent_component else {
        return 0;
    };
    parent_component.context
}

#[cfg_attr(rust_js, rust_js::nullable(parent_component, callback))]
fn update_container_impl(element: u32, parent_component: Option<&Component>, callback: Option<&dyn Fn(u32)>) -> u32 {
    let context = get_context_for_subtree(parent_component);
    if let Some(callback) = callback {
        callback(element + context);
    }
    element + context
}

pub fn update_container(element: u32, parent_component: Option<&Component>, callback: Option<&dyn Fn(u32)>) -> u32 {
    update_container_impl(element, parent_component, callback)
}

#[cfg_attr(rust_js, rust_js::nullable(parent_component))]
fn strict_context(parent_component: Option<&Component>) -> u32 {
    let Some(parent_component) = parent_component else {
        return 7;
    };
    parent_component.context
}

#[cfg_attr(rust_js, rust_js::nullable(parent_component))]
fn update_strict(element: u32, parent_component: Option<&Component>) -> u32 {
    element + strict_context(parent_component)
}

pub fn update_with(element: u32, parent: &Component) -> u32 {
    update_strict(element, Some(parent)) + update_strict(element, None)
}

fn main() {
    let parent = Component { context: 10 };
    println!("{}", update_container(1, None, None));
    println!("{}", update_container(1, Some(&parent), None));
    update_container(2, Some(&parent), Some(&|n| println!("called {n}")));
    update_container(3, None, Some(&|n| println!("called {n}")));
    println!("{}", update_with(1, &parent));
}
