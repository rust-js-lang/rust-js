// A struct literal's comments and blank lines, each where its field is
// written, React's noop renderer's host config: a comment before a field
// whose value is a block, `create_instance: Box::new({ let c = ..; move ||
// .. })`, is before the field, not the statement the object is in; a blank
// line before a field whose value is a constant or a closure that
// diverges, and after one whose value is a block, is kept; and a comment
// that a closure's lone tail starts with is in its body, `maySuspendCommit
// (type) { // Asks .. return ..; }`, and after its `=>`, not lost or put
// before the statement (rd296).
use std::cell::Cell;
use std::rc::Rc;

struct Config {
    supports_microtasks: bool,
    is_primary_renderer: bool,
    create_instance: Box<dyn Fn() -> f64>,
    may_suspend_commit: fn(&str) -> bool,
    get_instance_from_node: fn() -> f64,
    no_timeout: f64,
}

fn make() -> Config {
    let counter = Rc::new(Cell::new(0.0));
    Config {
        supports_microtasks: true,

        may_suspend_commit: |r#type| {
            // Asks whether it's possible for this combination of type and props
            // to ever need to suspend.
            r#type == "suspensey-thing"
        },

        // A comment before a field whose value is a block.
        create_instance: Box::new({
            let counter = Rc::clone(&counter);
            move || {
                let id = counter.get();
                counter.set(id + 1.0);
                id
            }
        }),

        is_primary_renderer: true,

        get_instance_from_node: || panic!("Not yet implemented."),

        no_timeout: -1.0,
    }
}

fn longer() -> impl Fn(&str) -> bool {
    |name| {
        // Longer than three.
        name.len() > 3
    }
}

fn main() {
    let config = make();
    println!(
        "{} {} {}",
        config.supports_microtasks, config.is_primary_renderer, config.no_timeout
    );
    println!("{} {}", (config.create_instance)(), (config.create_instance)());
    println!(
        "{} {}",
        (config.may_suspend_commit)("suspensey-thing"),
        (config.may_suspend_commit)("div")
    );
    let _ = config.get_instance_from_node;
    let longer = longer();
    println!("{} {}", longer("ab"), longer("abcd"));
}
