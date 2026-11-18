// An `Option` of text mapped by what gives the text as it is, `str::to_string`,
// `String::from`, `ToString::to_string` or a closure's `to_string()`: its
// value as JS has it, so `map_or_else(g, f)` is `option ?? g()` and `map(f)`
// the option itself, as the vercel-ui SDK's `locale ?? new
// Intl.DateTimeFormat().resolvedOptions().locale` reads (G4).
fn runtime() -> String {
    println!("runtime asked");
    String::from("en-US")
}

fn tag(locale: Option<&str>) -> String {
    locale.map_or_else(runtime, str::to_string)
}

fn from(locale: Option<&str>) -> String {
    locale.map_or_else(runtime, String::from)
}

fn through_the_trait(locale: Option<&str>) -> String {
    locale.map_or_else(runtime, ToString::to_string)
}

fn by_a_closure(locale: Option<&str>) -> String {
    locale.map_or_else(runtime, |locale| locale.to_string())
}

fn owned(locale: Option<&str>) -> Option<String> {
    locale.map(str::to_string)
}

fn main() {
    for locale in [Some("de-DE"), Some(""), None] {
        println!(
            "{} {} {} {} {:?}",
            tag(locale),
            from(locale),
            through_the_trait(locale),
            by_a_closure(locale),
            owned(locale)
        );
    }
}
