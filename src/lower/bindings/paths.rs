//! A binding's path, as JS writes one (ADR 1033): the global it starts
//! from, then each property, by its name, `console.log`, or by a quoted
//! key, `window['$RT']`. Quoted is how a program keeps a property's name
//! through a minifier that renames the others, Closure's ADVANCED mode,
//! where another script, the server's inline one, reads it by that name.

use crate::js::Expr;

/// A property a path reads, after the name it starts from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Step<'a> {
    /// `.log`: by its name, a JS name, which a minifier may rename.
    Named(&'a str),
    /// `['$RT']`: by a quoted key, which a minifier keeps.
    Quoted(&'a str),
}

impl Step<'_> {
    /// What it reads of `object`: `object.log`, or `object[key]` of a quoted key.
    pub(in crate::lower) fn of(self, object: Expr) -> Expr {
        match self {
            Step::Named(name) => Expr::member(object, name),
            Step::Quoted(key) => Expr::index(object, Expr::str(key)),
        }
    }
}

/// The name `path` starts from, `window` of `window['$RT']` and of
/// `window.$RT`.
pub(in crate::lower) fn first_name(path: &str) -> &str {
    &path[..path.find(['.', '[']).unwrap_or(path.len())]
}

/// Whether `path` reads a property, past the name it starts from.
pub(in crate::lower) fn has_steps(path: &str) -> bool {
    first_name(path).len() < path.len()
}

/// Each property `rest`, a path past its first name, reads, `.log` then
/// `['$RT']`; or why it isn't a path's.
pub(in crate::lower) fn steps(rest: &str) -> Result<Vec<Step<'_>>, String> {
    let mut steps = Vec::new();
    let mut rest = rest;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('.') {
            let name = first_name(after);
            if !crate::names::is_identifier(name) {
                return Err(format!(
                    "`.{name}` isn't a JS name's property: a key that isn't one is quoted, `['{name}']`"
                ));
            }
            steps.push(Step::Named(name));
            rest = &after[name.len()..];
        } else if let Some(after) = rest.strip_prefix("['") {
            let Some((key, after)) = after.split_once("']") else {
                return Err("a quoted key ends `']`".to_string());
            };
            if key.is_empty() || key.contains(['\'', '\\']) {
                return Err(format!(
                    "`['{key}']` isn't a key: one of at least one character, without a quote or a backslash"
                ));
            }
            steps.push(Step::Quoted(key));
            rest = after;
        } else {
            return Err(format!(
                "`{rest}` isn't a property: one is `.name` or a quoted key, `['name']`"
            ));
        }
    }
    Ok(steps)
}

/// What `path` reads of `object`, each of its properties in turn. Its
/// binding's path was checked, `validate`'s, so it's one.
pub(in crate::lower) fn read(object: Expr, path: &str) -> Expr {
    let steps = steps(path).unwrap_or_else(|why| panic!("a binding's path is checked where it's declared: {why}"));
    steps.into_iter().fold(object, |object, step| step.of(object))
}

/// Why `path`, the global path a binding reads, `console.log` or
/// `window['$RT']`, isn't one: it starts from a JS name.
pub(in crate::lower) fn global_refused(path: &str) -> Option<String> {
    let first = first_name(path);
    if !crate::names::is_identifier(first) {
        return Some(format!("`{first}` isn't a JS name, which a global's path starts from"));
    }
    steps(&path[first.len()..]).err()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_of_a_path() {
        assert_eq!(first_name("window['$RT']"), "window");
        assert_eq!(first_name("console.log"), "console");
        assert_eq!(
            steps(".a['-b'].c"),
            Ok(vec![Step::Named("a"), Step::Quoted("-b"), Step::Named("c")])
        );
        assert_eq!(steps("['worker-bundle']"), Ok(vec![Step::Quoted("worker-bundle")]));
        assert!(steps(".worker-bundle").is_err());
        assert!(steps("['$RT'").is_err());
        assert!(steps("[\"rt\"]").is_err());
        assert!(steps("['']").is_err());
        assert!(global_refused("window['$RT']").is_none());
        assert!(global_refused("['$RT']").is_some());
    }
}
