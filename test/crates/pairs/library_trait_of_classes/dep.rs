// A library's trait of classes (rd283), each of its impls a class's, whose
// methods are each class's prototype's: its consumer calls them as the
// library does, on the prototype, its own code's and its consumer's (#1073).
pub trait Render {
    fn label(&self) -> String;

    fn render(&self, times: u32) -> String {
        let mut out = String::new();
        for _ in 0..times {
            out.push_str(&self.label());
        }
        out
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Root {
    name: String,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Root {
    pub fn new(name: &str) -> Self {
        Root {
            name: name.to_string(),
        }
    }
}

impl Render for Root {
    fn label(&self) -> String {
        format!("<{}>", self.name)
    }
}

pub fn shown(item: &Root) -> String {
    item.render(2)
}

pub fn root(name: &str) -> Root {
    Root::new(name)
}
