// Mutations of crates/rust-js-lower/src/boundary/checks.rs (ADR 1192).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/boundary/checks.rs";

export const mutations: Mutation[] = [
  {
    name: "private-function-checked",
    breaks: "a function no JS caller can call checks what Rust gives it, so Rust's own calls pay for it (#2403)",
    file,
    find: "        crate::analysis::js_facing(self.tcx, self.krate.foreign, item)\n            && match item.as_local() {\n",
    replace: "        true\n            && match item.as_local() {\n",
    tests: ["test/boundary-checks.test.ts", "-t", "a private function"],
  },
  {
    name: "u32-checked-as-i32",
    breaks: "a `u32` parameter is checked as an `i32`, so `double(-1)` runs on 4294967294 (#2403)",
    file,
    find: '                Num::U32 => "$checkU32",',
    replace: '                Num::U32 => "$checkI32",',
    tests: ["test/boundary-checks.test.ts", "-t", "an integer of 32 bits"],
  },
  {
    name: "shared-borrow-unchecked",
    breaks: "a `&str` parameter isn't checked, so a lone surrogate reaches Rust as a `str` (#2403)",
    file,
    find: "            Contract::Shared(inner) => return self.shallow_check(inner),",
    replace: "            Contract::Shared(_) => return None,",
    tests: ["test/boundary-checks.test.ts", "-t", "a string is well-formed"],
  },
  {
    name: "fieldless-enum-unchecked",
    breaks: "a fieldless enum's parameter isn't checked, so `\"lost\"` is a variant no `match` arm has (#2403)",
    file,
    find: "            Contract::Local(did, _) if let Some(names) = self.fieldless_names(did) => {",
    replace: "            Contract::Local(did, _) if let Some(names) = self.fieldless_names(did).filter(|_| false) => {",
    tests: ["test/boundary-checks.test.ts", "-t", "an enum one of its names"],
  },
  {
    name: "option-none-refused",
    breaks: "a plain `Option` parameter allows no `None`, so `pad(\"tea\")` throws where Rust's `None` is meant (#2403)",
    file,
    find: '                Contract::Option(inner) => Some((self.shallow_check(inner)?, Some("nullish"))),',
    replace: "                Contract::Option(inner) => Some((self.shallow_check(inner)?, None)),",
    tests: ["test/boundary-checks.test.ts", "-t", "an Option's None"],
  },
  {
    name: "nullable-none-nullish",
    breaks: "a nullable parameter takes `undefined` as its `None`, which its body tests `!== null`, so it reads `undefined` as a string (#2403)",
    file,
    find: '            Form::Nullable(inner) => Some((self.shallow_check(inner)?, Some("null"))),',
    replace: '            Form::Nullable(inner) => Some((self.shallow_check(inner)?, Some("nullish"))),',
    tests: ["test/boundary-checks.test.ts", "-t", "an Option's None"],
  },
  {
    name: "forwarder-checked",
    breaks: "a function that only passes its parameters on to one that checks them checks them too, so its module writes a function where it re-exported the callee (rd575, #2403)",
    file,
    find: "            || self.forwards_to_a_checked_function(item, thir))",
    replace: ")",
    tests: ["test/snapshots.test.ts", "-t", "modules"],
  },
  {
    name: "async-pattern-checked",
    breaks: "an \`async fn\`'s pattern, which rustc names \`__arg0\`, is checked as if named plainly, so \`summed([a, b])\` loses its shape before its design is chosen (#2403)",
    file,
    find: "    matches!(ident, Some(Some(ident)) if !ident.name.as_str().starts_with(\"__arg\"))",
    replace: "    matches!(ident, Some(Some(_)))",
    tests: ["test/boundary-async.test.ts", "-t", "its pattern keeps its shape"],
  },
  {
    name: "js-callable-by-pub-alone",
    breaks: "every `pub fn` is called one JS may call, a `pub mod`'s too, which only Rust names: its checks and its split back, the port's +8.5% (#2403 lever (3))",
    file: "crates/rust-js-lower/src/boundary/checks.rs",
    find: "                Some(local) => self.krate.js_reached.may_call(self.tcx, local),\n",
    replace: "                Some(_) => true,\n",
    tests: ["test/boundary-reach.test.ts", "-t", "only pub mod paths reach is unchecked"],
  },
  {
    name: "library-target-taken-for-checked",
    breaks: "a forwarder to a library's function its own JS doesn't check is taken for one that does, so its module re-exports the library's in its place and JS calls it unchecked, `serverDecode(5, { id: -1 })` (#2403 (Y))",
    file: "crates/rust-js-lower/src/boundary/checks.rs",
    find: "                None => self.krate.foreign.item(item).is_none_or(|imported| imported.checked),\n",
    replace: "                None => true,\n",
    tests: ["test/declarations-libraries.test.ts", "-t", "one instantiation is its re-export"],
  },
];
