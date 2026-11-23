//! A call of a runtime helper, read by the names its own JS declares its
//! parameters by (`runtime::params`, #2334): `call.arg("cell")`, not
//! `[cell, raised]`, so an argument a helper gains, an optional one a call
//! gives, or one a call is padded with leaves each reader reading what it
//! asks for. Where it's raised, `raisedAt`, is an argument like any other:
//! read by its name, and compared as a value where calls are compared, as
//! two of one place raised at two columns are two panics (#2334's step 1).
//! A name its helper doesn't declare is a bug, said where it's asked.

use super::*;
use crate::runtime::Param;

/// A call of the helper `name`, its arguments read by its parameters.
#[derive(Clone, Copy)]
pub struct HelperCall<'e> {
    name: &'static str,
    params: &'static [Param],
    args: &'e [Expr],
}

/// `HelperCall`'s, its arguments written in place.
pub struct HelperCallMut<'e> {
    name: &'static str,
    params: &'static [Param],
    args: &'e mut Vec<Expr>,
}

/// Where `param` is in `name`'s parameters, which it must declare.
fn position(name: &str, params: &[Param], param: &str) -> usize {
    (params.iter().position(|each| each.name == param))
        .unwrap_or_else(|| panic!("the helper {name} declares no parameter `{param}`"))
}

/// What a call gives at `at`: `None` where it gives nothing there, or an
/// `undefined` only so a later one is where it's declared (`Known::Padded`).
fn given_at(args: &[Expr], at: usize) -> Option<&Expr> {
    args.get(at).filter(|arg| !arg.facts.has(Known::Padded))
}

impl Expr {
    /// This, a call of the helper `name`, read by its parameters; `None` of
    /// anything else.
    pub fn helper_call_of(&self, name: &'static str) -> Option<HelperCall<'_>> {
        self.helper_call_of_any(&[name])
    }

    /// This, a call of one of `names`, read by its parameters.
    pub fn helper_call_of_any(&self, names: &[&'static str]) -> Option<HelperCall<'_>> {
        let ExprKind::Call(callee, args) = &self.kind else {
            return None;
        };
        let name = *names.iter().find(|&&name| callee.helper_name() == Some(name))?;
        let params = crate::runtime::params(name).unwrap_or_else(|| panic!("{name} is no helper's function"));
        Some(HelperCall { name, params, args })
    }

    /// `helper_call_of`'s, its arguments written in place.
    pub fn helper_call_of_mut(&mut self, name: &'static str) -> Option<HelperCallMut<'_>> {
        let ExprKind::Call(callee, args) = &mut self.kind else {
            return None;
        };
        if callee.helper_name() != Some(name) {
            return None;
        }
        let params = crate::runtime::params(name).unwrap_or_else(|| panic!("{name} is no helper's function"));
        Some(HelperCallMut { name, params, args })
    }
}

impl<'e> HelperCall<'e> {
    /// Which helper it calls.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// What it gives for `param`; `None` where it gives nothing for it.
    pub fn arg(&self, param: &str) -> Option<&'e Expr> {
        given_at(self.args, position(self.name, self.params, param))
    }

    /// Whether it gives anything for `param`.
    pub fn given(&self, param: &str) -> bool {
        self.arg(param).is_some()
    }

    /// What it gives from `param` on, its rest's items.
    pub fn args_from(&self, param: &str) -> &'e [Expr] {
        let at = position(self.name, self.params, param);
        &self.args[at.min(self.args.len())..]
    }
}

impl<'e> HelperCallMut<'e> {
    /// What it gives for `param`, to write; `None` where it gives nothing.
    /// It borrows the view, which reads others after it.
    pub fn arg_mut(&mut self, param: &str) -> Option<&mut Expr> {
        let at = position(self.name, self.params, param);
        self.args.get_mut(at).filter(|arg| !arg.facts.has(Known::Padded))
    }

    /// `arg_mut`'s, the view given up for it, so what it gives is the
    /// call's to hold as long as the call is, as a function returns it: not
    /// a second way to `arg_mut`, but the split std makes between
    /// `OccupiedEntry::get_mut` and `into_mut` (ADR 1186).
    pub fn into_arg_mut(self, param: &str) -> Option<&'e mut Expr> {
        let at = position(self.name, self.params, param);
        self.args.get_mut(at).filter(|arg| !arg.facts.has(Known::Padded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_is_read_by_its_parameters_names() {
        let call = Expr::call(Expr::helper("$lent"), vec![Expr::var("it")]);
        let lent = call.helper_call_of("$lent").expect("a call of `$lent`");
        assert!(lent.given("it") && !lent.given("kind"));
        assert!(call.helper_call_of("$index").is_none());
        let padded = Expr::call(
            Expr::helper("$count"),
            vec![
                Expr::var("it"),
                Expr {
                    facts: Expr::js_undefined().facts.with(Known::Padded),
                    ..Expr::js_undefined()
                },
            ],
        );
        let count = padded.helper_call_of("$count").expect("a call of `$count`");
        assert!(count.given("it") && !count.given("drop"));
    }

    /// Each name a call is read by, `.arg("cell")`, `.given("drop")`, is
    /// one its helper declares, the nearest `helper_call_of` before it, in
    /// 30 lines, names, of every file of the compiler's: a helper's
    /// parameter renamed fails here, naming the reader.
    #[test]
    fn each_name_a_call_is_read_by_is_one_its_helper_declares() {
        fn sources(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("a directory of sources").flatten() {
                let path = entry.path();
                match path.is_dir() {
                    true => sources(&path, found),
                    false if path.extension().is_some_and(|e| e == "rs") => found.push(path),
                    false => {}
                }
            }
        }
        // The workspace's members, its manifest's one list, and the binary's
        // own sources at its root.
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
        let root = manifest.parent().expect("the workspace's root");
        let text = std::fs::read_to_string(&manifest).expect("the workspace's manifest");
        let members = text
            .lines()
            .find_map(|line| line.strip_prefix("members = "))
            .expect("its members");
        let mut found = Vec::new();
        for dir in members
            .split('"')
            .skip(1)
            .step_by(2)
            .map(|member| format!("{member}/src"))
            .chain(["src".to_string()])
        {
            sources(&root.join(dir), &mut found);
        }
        let quoted = |text: &str| -> Vec<String> { text.split('"').skip(1).step_by(2).map(str::to_string).collect() };
        let mut wrong = Vec::new();
        for file in found.iter().filter(|file| !file.ends_with("js/helper_call.rs")) {
            let text = std::fs::read_to_string(file).expect("a source");
            let lines: Vec<&str> = text.lines().collect();
            for (at, line) in lines.iter().enumerate() {
                for reader in [
                    ".arg(\"",
                    ".given(\"",
                    ".args_from(\"",
                    ".arg_mut(\"",
                    ".into_arg_mut(\"",
                ] {
                    for (i, _) in line.match_indices(reader) {
                        let param = quoted(&line[i..]).into_iter().next().unwrap_or_default();
                        let named = (at.saturating_sub(30)..=at).rev().find_map(|j| {
                            let line = lines[j];
                            line.find("helper_call_of").map(|k| {
                                quoted(&line[k..])
                                    .into_iter()
                                    .take_while(|n| crate::runtime::params(n).is_some())
                                    .collect::<Vec<_>>()
                            })
                        });
                        let place = format!("{}:{}", file.display(), at + 1);
                        match named {
                            Some(helpers) if !helpers.is_empty() => {
                                for helper in helpers {
                                    let declared = crate::runtime::params(&helper)
                                        .is_some_and(|ps| ps.iter().any(|p| p.name == param));
                                    if !declared {
                                        wrong.push(format!("{place}: {helper} declares no parameter `{param}`"));
                                    }
                                }
                            }
                            // Another's, a `Command`'s `.arg("-vV")`.
                            _ => {}
                        }
                    }
                }
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// `call_located`'s pads are no argument given: `$count(it)` asked
    /// where it's raised, `$count(it, undefined, "lib.rs:1:1")`, gives no
    /// `drop`.
    #[test]
    fn a_located_call_s_pads_are_not_given() {
        let call = Expr::raised_at(Expr::str("lib.rs:1:1"), || {
            Expr::call_located(Expr::helper("$count"), vec![Expr::var("it")], true)
        });
        let count = call.helper_call_of("$count").expect("a call of `$count`");
        assert!(!count.given("drop") && count.given("raisedAt"));
    }

    /// What `into_arg_mut` gives is the call's, written through after the
    /// view is gone.
    #[test]
    fn an_argument_given_up_is_written_in_its_call() {
        let mut call = Expr::call(Expr::helper("$cleanup"), vec![Expr::var("run"), Expr::var("e")]);
        let run = (call.helper_call_of_mut("$cleanup")).and_then(|cleanup| cleanup.into_arg_mut("run"));
        *run.expect("its `run`") = Expr::var("drops");
        let cleanup = call.helper_call_of("$cleanup").expect("a call of `$cleanup`");
        assert!(same_path(cleanup.arg("run").expect("its `run`"), &Expr::var("drops")));
    }

    #[test]
    #[should_panic(expected = "declares no parameter `kinds`")]
    fn a_name_its_helper_doesnt_declare_is_a_bug() {
        let call = Expr::call(Expr::helper("$lent"), vec![Expr::var("it")]);
        call.helper_call_of("$lent").map(|lent| lent.given("kinds"));
    }
}
