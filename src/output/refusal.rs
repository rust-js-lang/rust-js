//! Why a module's tree is refused before it's printed (`js::validate`,
//! #1681 change 3, M1): its first problem, and what broke it, the lowering
//! or preparation's pass that did, a bug of rust-js's either way.

use std::path::Path;

use rust_js_program::js;

/// Why `module`, prepared with `passes` from `lowered`, as it was lowered,
/// isn't printed to `js_path`; none where it checks.
pub(super) fn refusal(
    js_path: &Path,
    lowered: &js::Module,
    module: &js::Module,
    passes: rust_js_program::prepare::Passes,
) -> Option<String> {
    let problem = js::validate::module(module).into_iter().next()?;
    let broken = |module: &js::Module| js::validate::module(module).contains(&problem);
    let by = match broken(lowered) {
        true => "as it was lowered".to_string(),
        false => match rust_js_program::prepare::breaking_pass(lowered, passes, &broken) {
            Some(pass) => format!("by preparation's `{pass}`"),
            None => "by preparation".to_string(),
        },
    };
    Some(format!(
        "rust-js made {} wrong {by}: {problem}: a bug of rust-js's",
        js_path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_js_program::js::{Expr, Item, Module, StmtKind};

    // A read nothing declares nor made a global, `console` as a local's
    // read, is refused, naming the module, the read and what broke it: the
    // lowering, where its tree had it before preparation. Of a tree that
    // checks, nothing (#1681, ADR 1143).
    #[test]
    fn a_read_nothing_declares_is_refused_naming_what_broke_it() {
        let module = |read: Expr| Module {
            items: vec![Item::Statements(vec![StmtKind::Expr(read).at(js::Span::NONE)])],
            ..Module::default()
        };
        let passes = rust_js_program::prepare::Passes::default();
        let path = Path::new("dist/main.js");
        let unresolved = module(Expr::var("console"));
        assert_eq!(
            refusal(path, &unresolved, &unresolved, passes).as_deref(),
            Some(
                "rust-js made dist/main.js wrong as it was lowered: `console` is read, and nothing declares it nor made it a global: a bug of rust-js's"
            )
        );
        let global = module(Expr::global("console"));
        assert_eq!(refusal(path, &global, &global, passes), None);
    }
}
