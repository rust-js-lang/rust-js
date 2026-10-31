// An untagged enum a variant of another holds, React's noop renderer's
// childToJSX's `Instance | TextInstance | Array<Instance | TextInstance>`:
// the outer variant is told by the inner's kinds together, an object no
// array is, and the inner's variant as it's told alone,
// `Array.isArray(child.children)` (rd305).
use std::cell::RefCell;
use std::rc::Rc;

struct Instance {
    #[cfg_attr(rust_js, rust_js::test)]
    children: Rc<RefCell<Vec<Child>>>,
    tag: String,
}

struct TextInstance {
    text: String,
}

#[derive(Clone)]
#[cfg_attr(rust_js, rust_js::untagged)]
enum Child {
    Instance(Rc<Instance>),
    Text(Rc<TextInstance>),
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum JsxChild {
    Child(Child),
    Children(Rc<RefCell<Vec<Child>>>),
}

// A container, told by its `rootID`, whose `children` an instance's test
// reads too: that tells nothing apart here, so an item is an object no
// container is, or a number (rd291's rule, of each of the inner's kinds).
struct Container {
    #[cfg_attr(rust_js, rust_js::name = "rootID", rust_js::test)]
    root_id: String,
    children: Rc<RefCell<Vec<Child>>>,
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Item {
    Instance(Rc<Instance>),
    Count(f64),
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Parent {
    Item(Item),
    Container(Rc<Container>),
}

// A number or a word, as a key is, beside a list of either.
#[cfg_attr(rust_js, rust_js::untagged)]
enum Key {
    Index(f64),
    Name(String),
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Keys {
    One(Key),
    Many(Vec<Key>),
    Missing(bool),
}

fn describe(child: JsxChild) -> String {
    match child {
        JsxChild::Children(children) => {
            let children = children.borrow().clone();
            children
                .into_iter()
                .map(|c| describe(JsxChild::Child(c)))
                .collect::<Vec<_>>()
                .join(",")
        }
        JsxChild::Child(Child::Instance(instance)) => {
            format!(
                "<{}>{}",
                instance.tag,
                describe(JsxChild::Children(Rc::clone(&instance.children)))
            )
        }
        JsxChild::Child(Child::Text(text)) => text.text.clone(),
    }
}

fn is_one(child: &JsxChild) -> bool {
    matches!(child, JsxChild::Child(_))
}

fn parent_name(parent: &Parent) -> String {
    match parent {
        Parent::Item(Item::Instance(instance)) => instance.tag.clone(),
        Parent::Item(Item::Count(count)) => format!("{count} items"),
        Parent::Container(container) => {
            format!("{} of {}", container.root_id, container.children.borrow().len())
        }
    }
}

fn key(key: &Key) -> String {
    match key {
        Key::Index(i) => format!("#{i}"),
        Key::Name(name) => name.clone(),
    }
}

fn keys(keys: &Keys) -> String {
    match keys {
        Keys::One(one) => key(one),
        Keys::Many(many) => many.iter().map(key).collect::<Vec<_>>().join("+"),
        Keys::Missing(missing) => format!("missing {missing}"),
    }
}

fn main() {
    let text = Child::Text(Rc::new(TextInstance {
        text: "hi".to_string(),
    }));
    let span = Child::Instance(Rc::new(Instance {
        children: Rc::new(RefCell::new(vec![text.clone()])),
        tag: "span".to_string(),
    }));
    let div = Child::Instance(Rc::new(Instance {
        children: Rc::new(RefCell::new(vec![span.clone(), text.clone()])),
        tag: "div".to_string(),
    }));
    println!("{}", describe(JsxChild::Child(div.clone())));
    println!("{}", describe(JsxChild::Child(text.clone())));
    let root = Rc::new(Container {
        root_id: "root".to_string(),
        children: Rc::new(RefCell::new(vec![div.clone()])),
    });
    println!("{}", parent_name(&Parent::Container(root)));
    let Child::Instance(instance) = &div else {
        unreachable!()
    };
    println!("{}", parent_name(&Parent::Item(Item::Instance(Rc::clone(instance)))));
    println!("{}", parent_name(&Parent::Item(Item::Count(2.0))));
    let all = Rc::new(RefCell::new(vec![div, span, text.clone()]));
    println!("{}", describe(JsxChild::Children(Rc::clone(&all))));
    println!("{}", describe(JsxChild::Children(Rc::new(RefCell::new(Vec::new())))));
    println!(
        "{} {}",
        is_one(&JsxChild::Child(text)),
        is_one(&JsxChild::Children(all))
    );
    println!("{}", keys(&Keys::One(Key::Index(3.0))));
    println!("{}", keys(&Keys::One(Key::Name("a".to_string()))));
    println!(
        "{}",
        keys(&Keys::Many(vec![Key::Name("b".to_string()), Key::Index(1.5)]))
    );
    println!("{}", keys(&Keys::Missing(true)));
}
