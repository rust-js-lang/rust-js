// Mutations of src/lower/drops/uses.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "drops-by-need-passed-ignored",
    breaks: "a drop passed on to a function that uses it isn't kept, so `discard` drops nothing",
    file: "src/lower/drops/uses.rs",
    find: "                if kept.contains(&to) && kept.insert(from) {\n",
    replace: "                if false && kept.contains(&to) && kept.insert(from) {\n",
    tests: ["test/crates.test.ts", "-t", "drops_by_need"],
  },
  {
    name: "drops-by-need-uses-unnoted",
    breaks: "a drop a body uses isn't kept, so `pick` drops nothing",
    file: "src/lower/drops/uses.rs",
    find: "        observed\n            .used\n            .extend(self.drop_state.used_drops.iter().map(|&param| (id, param)));\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "drops_by_need"],
  },
  {
    name: "drops-by-need-passed-as-used",
    breaks: "a drop passed on counts as used, so `relay` keeps one `keep` doesn't use",
    file: "src/lower/drops/uses.rs",
    find: "        if !self.in_copied_default() && self.krate.takes_used_drops.contains(self.item) {\n",
    replace: "        if false && !self.in_copied_default() && self.krate.takes_used_drops.contains(self.item) {\n",
    tests: ["test/crates.test.ts", "-t", "takes the drops its body uses"],
  },
  {
    name: "drops-by-need-params-kept",
    breaks: "a function keeps the drop parameters it doesn't use",
    file: "src/lower/drops/uses.rs",
    find: "if unused.contains(name.as_str())));\n",
    replace: "if false && unused.contains(name.as_str())));\n",
    tests: ["test/crates.test.ts", "-t", "takes the drops its body uses"],
  },
  {
    name: "drops-by-need-arguments-kept",
    breaks: "a call still gives a drop its callee doesn't take, before one it does, as `discard`'s of `pick`",
    file: "src/lower/drops/uses.rs",
    find: "kept.contains(&(local(item), index)).then_some(*drop)",
    replace: "Some(if kept.contains(&(local(item), index)) { *drop } else { Expr::undefined() })",
    tests: ["test/crates.test.ts", "-t", "drops_by_need"],
  },
  {
    name: "drops-by-need-arrow-kept",
    breaks: "a function given as a value stays an arrow that passes its parameters on, `(arg0) => keep(arg0)`",
    file: "src/lower/drops/uses.rs",
    find: "            passed.then(|| (**callee).clone())\n",
    replace: "            passed.then(|| (**callee).clone()).filter(|_| false)\n",
    tests: ["test/crates.test.ts", "-t", "takes the drops its body uses"],
  },
  {
    name: "generic-drop-not-given",
    breaks: "a generic function given a value with a destructor isn't given its drop, and never drops it",
    file: "src/lower/drops/uses.rs",
    find: "        let drop = drop?.unwrap_or_else(Expr::undefined);\n",
    replace: "        let drop = drop.map(|_| Expr::undefined())?;\n",
    tests: ["test/corpus.test.ts", "-t", "drop_generic"],
  },
];
