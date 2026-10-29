// A function called through a module path the program writes, with no
// `use` of the function, is read through the module's namespace,
// `import * as Plugin from "./plugin.js"` and `Plugin.extractEvents(..)`,
// as React DOM's DOMPluginEventSystem writes
// `EnterLeaveEventPlugin.extractEvents(..)`; one the program `use`s is
// its named import. A namespace another name of the module's takes is
// given apart, and a path through `crate`, `self` or `super` alone names
// no module of its own (rd114).

mod plugin {
    pub fn extract_events(n: u32) -> u32 {
        n * 2
    }

    pub fn name() -> &'static str {
        "plugin"
    }

    pub fn doubled(n: &u32) -> u32 {
        n * 2
    }

    pub mod nested {
        pub fn depth() -> u32 {
            super::extract_events(1) + super::super::plugin::extract_events(2)
        }
    }
}

mod other {
    pub fn extract_events(n: u32) -> u32 {
        n + 1
    }
}

use plugin::name;

struct Other {
    seen: u32,
}

impl Other {
    fn seen(&self) -> u32 {
        self.seen
    }
}

fn extract_events(n: u32) -> u32 {
    plugin::extract_events(n) + other::extract_events(n)
}

fn main() {
    let other = Other { seen: 3 };
    let doubled: Vec<u32> = [1, 2, 3].iter().map(plugin::doubled).collect();
    println!(
        "{} {} {} {:?} {}",
        extract_events(other.seen()),
        name(),
        self::plugin::name(),
        doubled,
        plugin::nested::depth(),
    );
}
