//! A library's struct of its own functions, read through its thread-local,
//! as React's `Children`: a fn pointer type whose signature holds a `dyn Fn`.
pub struct Children {
    pub for_each: fn(Option<&'static str>, &dyn Fn(u32), Option<&'static str>),
    pub count: fn(Option<&'static str>) -> u32,
}

fn for_each(children: Option<&'static str>, f: &dyn Fn(u32), context: Option<&'static str>) {
    for i in 0..count(children) {
        f(i);
    }
    if let Some(context) = context {
        println!("context {context}");
    }
}

fn count(children: Option<&'static str>) -> u32 {
    children.map_or(0, |children| children.len() as u32)
}

thread_local! {
    pub static CHILDREN: Children = const { Children { for_each, count } };
}

/// One a consumer gives it, which may be any function, JS's too.
pub struct Hooks {
    pub on_each: fn(u32, Option<u32>) -> u32,
}

pub fn hooks(on_each: fn(u32, Option<u32>) -> u32) -> Hooks {
    Hooks { on_each }
}
