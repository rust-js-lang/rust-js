// A struct JS made, a `js::cast` of what it's given: each of its fields
// read as often as Rust reads it, as a getter may answer it (ADR 1022,
// #430), in a `let`, an `if let`, a guard's arm, a borrow ended and a
// loop's test.
//@ call: restore [{}, {"style": {"margin": "4px", "padding": "2px"}}]
//@ call: restore [{}, {"style": {"padding": "2px"}}]
//@ call: marginOf [{}, {"style": {"margin": "4px", "padding": "2px"}}]
//@ call: marginOf [{}, {"style": {"padding": 0}}]
//@ call: copyChild [{"child": null}, {"child": {"tag": 1}}]
//@ call: depth [{"parent": {"parent": {"parent": null}}}]
//@ call: total [{"items": [1, 2, 3], "scale": 2}]

use js::Unknown;
use std::cell::RefCell;

pub struct StyleProp {
    pub margin: Option<&'static Unknown>,
    pub padding: Option<&'static Unknown>,
}

// Safety: what JS gives is an object of these fields.
unsafe impl js::Defined for StyleProp {}

#[cfg_attr(rust_js, rust_js::link_name = "set margin")]
#[cfg_attr(rust_js, rust_js::runs_nothing)]
fn set_margin(this: &Unknown, value: Option<&Unknown>) {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set padding")]
#[cfg_attr(rust_js, rust_js::runs_nothing)]
fn set_padding(this: &Unknown, value: Option<&Unknown>) {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "Boolean")]
fn truthy(value: &Unknown) -> bool {
    unreachable!()
}

fn apply(style: &Unknown, style_prop: &StyleProp) {
    let margin = style_prop.margin;
    if margin.is_some() {
        set_margin(style, margin);
    } else {
        let padding = style_prop.padding;
        set_padding(style, padding);
    }
}

pub fn restore(style: &Unknown, props: &Unknown) {
    // Safety: a style prop is an object of these fields.
    let style_prop: Option<&StyleProp> = unsafe { js::cast(js::get(props, "style")) };
    if let Some(style_prop) = style_prop {
        apply(style, style_prop);
    }
}

pub fn margin_of(style: &Unknown, props: &Unknown) {
    // Safety: a style prop is an object of these fields.
    let view: &StyleProp = unsafe { js::cast(js::get(props, "style")) };
    if let Some(margin) = view.margin {
        set_margin(style, Some(margin));
        if let Some(again) = view.margin {
            set_margin(style, Some(again));
        }
    }
    match view.padding {
        Some(padding) if truthy(padding) => set_padding(style, Some(padding)),
        Some(padding) => set_margin(style, Some(padding)),
        None => {}
    }
}

pub struct Fiber {
    pub child: RefCell<Option<&'static Unknown>>,
}

// Safety: what JS gives is an object of this field.
unsafe impl js::Defined for Fiber {}

pub fn copy_child(target: &Unknown, current: &Unknown) {
    // Safety: what's given is a fiber.
    let target: &Fiber = unsafe { js::cast(Some(target)) };
    let current: &Fiber = unsafe { js::cast(Some(current)) };
    let child = current.child.borrow().clone();
    target.child.replace(child);
}

pub struct Node {
    pub parent: Option<&'static Node>,
}

// Safety: what JS gives is an object of this field.
unsafe impl js::Defined for Node {}

pub fn depth(node: &Unknown) -> u32 {
    // Safety: what's given is a node.
    let mut node: &Node = unsafe { js::cast(Some(node)) };
    let mut depth = 0;
    while let Some(parent) = node.parent {
        depth += 1;
        node = parent;
    }
    depth
}

pub struct Order {
    pub items: Vec<f64>,
    pub scale: f64,
}

// Safety: what JS gives is an object of these fields.
unsafe impl js::Defined for Order {}

pub fn total(order: &Unknown) -> f64 {
    // Safety: what's given is an order.
    let order: &Order = unsafe { js::cast(Some(order)) };
    order.items.iter().map(|item| item * order.scale).sum()
}
