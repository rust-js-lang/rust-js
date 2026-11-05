// An `Option`'s `filter` by the truth of what it holds, then its fallback,
// React's mountLazyComponent's `getComponentNameFromType(Component) ||
// Component` (#787): `name.filter(|name| !name.is_empty())`, then
// `unwrap_or_else(|| component)` or `map_or_else(|| component, f)` where
// `f` gives what it's given, is JS's `||` of the two, read once.

fn name_of(n: u32) -> Option<String> {
    match n {
        0 => None,
        1 => Some(String::new()),
        _ => Some(format!("Name{n}")),
    }
}

fn unwrapped(n: u32, component: &str) -> String {
    name_of(n)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| component.to_string())
}

fn mapped(n: u32, component: &str) -> String {
    name_of(n)
        .filter(|name| !name.is_empty())
        .map_or_else(|| component.to_string(), |name| name)
}

// An `f` that isn't the value as it is is called, as Rust calls it.
fn shouted(n: u32, component: &str) -> String {
    name_of(n)
        .filter(|name| !name.is_empty())
        .map_or_else(|| component.to_string(), |name| name.to_uppercase())
}

// A number's `!= 0.0` isn't JS's truth of it, which `NaN` fails too: no
// `||`, each value as Rust has it.
fn nonzero(x: Option<f64>) -> f64 {
    x.filter(|x| *x != 0.0).map_or_else(|| -1.0, |x| x)
}

fn nonzero_int(x: Option<i32>) -> i32 {
    x.filter(|x| *x != 0).map_or_else(|| -1, |x| x)
}

struct Props {
    name: Option<String>,
}

fn named(props: &Props) -> Option<&str> {
    props.name.as_deref().filter(|&name| name != "auto")
}

fn main() {
    for n in 0..3 {
        println!("{} {} {}", unwrapped(n, "Component"), mapped(n, "Component"), shouted(n, "Component"));
    }
    for x in [None, Some(0.0), Some(-0.0), Some(f64::NAN), Some(2.5)] {
        println!("{}", nonzero(x));
    }
    for x in [None, Some(0), Some(-3)] {
        println!("{}", nonzero_int(x));
    }
    for name in [None, Some(""), Some("auto"), Some("slide")] {
        let props = Props { name: name.map(str::to_string) };
        println!("{:?}", named(&props));
    }
}
