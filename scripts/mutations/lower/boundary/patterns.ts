// Mutations of crates/rust-js-lower/src/boundary/patterns.rs (ADR 1192, amended).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/boundary/patterns.rs";

export const mutations: Mutation[] = [
  {
    name: "pattern-parts-unchecked",
    breaks: "what a pattern parameter binds isn't checked, so `area([2, -3])` reaches Rust with a `u32` of -3 (#2403)",
    file,
    find: "            self.part_checks((&signature, pattern, body), ty, place, &mut out);",
    replace: "            let _ = (place, ty, body);",
    tests: ["test/boundary-patterns.test.ts", "-t", "a tuple's pattern"],
  },
  {
    name: "struct-pattern-unchecked",
    breaks: "a struct pattern's fields aren't checked by their keys, so `sized({ w: 2 })` reads an `h` of undefined (#2403)",
    file,
    find: "                let Shape::Object(fields) = self.shape(ty) else { return };",
    replace: "                let Shape::Object(fields) = self.shape(ty) else { return };\n                let fields: Vec<(crate::bindings::FieldKey, Ty<'tcx>)> = { let _ = fields; Vec::new() };",
    tests: ["test/boundary-patterns.test.ts", "-t", "a struct's pattern"],
  },
  {
    name: "pattern-callback-unchecked",
    breaks: "a callback a pattern binds isn't checked, so a \`rule\` giving -1 reaches Rust as a \`u32\` (#2403)",
    file,
    find: "        if let Some(callback) = self.callback_of(signature, ty) {",
    replace: "        if let Some(callback) = None::<super::callbacks::Callback> {",
    tests: ["test/boundary-patterns.test.ts", "-t", "a callback a pattern binds"],
  },
  {
    name: "unread-part-checked",
    breaks: "a part JS binds only to keep it out of a `...rest`, `lang: _`, is checked though Rust never reads it, and named apart, `lang: _lang` (#2403)",
    file: "crates/rust-js-lower/src/boundary/patterns.rs",
    find: "            js::Pattern::Name(name) if js::mentions_of(body, name) == 0 && !self.has_drops(erased(self.tcx, ty)) => {}",
    replace: "            js::Pattern::Name(name) if false && js::mentions_of(body, name) == 0 && !self.has_drops(erased(self.tcx, ty)) => {}",
    tests: ["test/jsx-props.test.ts","-t","a let pattern's flattened field is the rest"],
  },
  {
    name: "checks-unfolded",
    breaks: "a function's checks are each a statement, its place written once each, where one `$checks` writes it once (#2403, the lead's (A))",
    file: "crates/rust-js-lower/src/boundary/patterns.rs",
    find: "        folded(&function, out)\n",
    replace: "        out\n",
    tests: ["test/boundary-checks.test.ts", "-t", "what holds other values is checked whole"],
  },
];
