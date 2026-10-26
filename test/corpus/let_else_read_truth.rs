// A `let … else` of what `filter` keeps by its truth, of what's no
// variable, a call's, is it in its `const`, tested by its truth, as React's
// legacy context reads `type.contextTypes` (ADR 0635, amended): past the
// test it's what was given, an empty text or a `false` sent away first.
// Where what's sent away reads the `Option` it kept, `None`, it's kept.
struct Config {
    name: Option<&'static str>,
    enabled: Option<bool>,
}

fn name_of(config: &Config) -> Option<&'static str> {
    config.name
}

fn label(config: &Config) -> String {
    let Some(name) = name_of(config).filter(|name| !name.is_empty()) else {
        return "none".to_string();
    };
    format!("<{name}>")
}

fn kept(config: &Config) -> String {
    let kept = config.name.filter(|name| !name.is_empty());
    let Some(name) = kept else {
        return format!("{kept:?}");
    };
    name.to_uppercase()
}

fn enabled(config: &Config) -> u32 {
    let Some(on) = config.enabled.filter(|&on| on) else {
        return 0;
    };
    if on { 1 } else { 2 }
}

fn main() {
    let configs = [
        Config {
            name: Some("app"),
            enabled: Some(true),
        },
        Config {
            name: Some(""),
            enabled: Some(false),
        },
        Config {
            name: None,
            enabled: None,
        },
    ];
    for config in &configs {
        println!("{} {} {}", label(config), kept(config), enabled(config));
    }
}
