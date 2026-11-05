// An `Option` a combinator reads twice, held in a `const`: named by the
// call that made it unless the program owns that name, `finisher`, then
// by the closure's parameter that binds it, `const onFinish`; and where
// an `Option`'s own method made it, `as_deref()` or `filter()`, which
// gives back the option it had, `option`, never the method's name (#784,
// #769, ADR 0569).
pub struct Shared {
    finish: Option<fn(u32) -> u32>,
}

fn finisher(shared: &Shared) -> Option<fn(u32) -> u32> {
    shared.finish
}

fn double(n: u32) -> u32 {
    n * 2
}

fn finished(shared: &Shared, n: u32) -> Option<u32> {
    finisher(shared).map(|on_finish| on_finish(n))
}

pub struct Props {
    name: Option<String>,
}

fn pair_name(props: &Props) -> &str {
    let Some(name) = props.name.as_deref().filter(|&name| name != "auto") else {
        return "none";
    };
    name
}

fn name_from(n: u32) -> Option<&'static str> {
    if n > 0 { Some("Button") } else { Some("") }
}

fn shown(name: &str) -> String {
    name.to_string()
}

fn component_name(n: u32) -> String {
    name_from(n)
        .filter(|name| !name.is_empty())
        .map_or_else(|| "Component".to_string(), shown)
}

fn main() {
    let shared = Shared { finish: Some(double) };
    println!("{:?}", finished(&shared, 4));
    println!("{:?}", finished(&Shared { finish: None }, 4));
    for name in [Some("auto".to_string()), Some("pair".to_string()), None] {
        println!("{}", pair_name(&Props { name }));
    }
    for n in [0, 1] {
        println!("{}", component_name(n));
    }
}
