// Mutations of src/js/scopes.rs (ADR 0093). Its reads resolved
// innermost first, and a rename refused of a read from outside or one that
// passes the new name's declaration, are its unit tests', `cargo test`: no
// Rust rust-js lowers reaches them, as it numbers names in order.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "scopes-twice-allowed",
    breaks: "`const n = items.pop(); const n = ..` in one block, which JS refuses",
    file: "src/js/scopes.rs",
    find: "        if declaring.iter().any(|&s| self.declares(s, to)) {",
    replace: "        if false {",
    tests: ["test/corpus.test.ts", "-t", "^atomics.rs$"],
  },
  {
    name: "scopes-not-hoisted",
    breaks: "`const m = n; const n = 1;` in a block, whose `n` JS reads before it's made",
    file: "src/js/scopes.rs",
    find: "                StmtKind::Const(name, _) | StmtKind::Let(name, _) => self.bind(name),",
    replace: "                StmtKind::Const(..) | StmtKind::Let(..) => {}",
    tests: ["test/lowering-names.test.ts", "-t", "reuses a name"],
  },
  {
    name: "scopes-closure-params-undeclared",
    breaks: "a closure's parameter `json` isn't its own, so a `json$1` in it is named `json` where it reads the outer one",
    file: "src/js/scopes.rs",
    find: "        for param in params {\n            self.pattern(param);\n        }\n        self.block_in_scope(body);",
    replace: "        self.block_in_scope(body);",
    tests: ["test/snapshots.test.ts", "-t", "dynamic"],
    snapshots: true,
  },
  {
    name: "scopes-other-read-captured",
    breaks: "`const m = n;` in a block whose `n$1` is named `n`, which the read then is",
    file: "src/js/scopes.rs",
    find: "        of_from.iter().all(|r| r.of.is_some() && !passes(r, to)) && of_to.iter().all(|r| !passes(r, from))",
    replace: "        of_from.iter().all(|r| r.of.is_some() && !passes(r, to))",
    tests: ["test/lowering-names.test.ts", "-t", "reuses a name"],
  },
  {
    name: "on-load-scopes-unwalked",
    breaks: "names in closures `js::on_load!` sets keep the names given apart, `fiber$1`, `sortedNames$1` (ADR 0952)",
    file: "src/js/scopes.rs",
    find: "                self.tree.module_scope = true;\n                self.block_in_scope(stmts);\n",
    replace: "                let _ = stmts;\n                self.tree.twice.push(String::new());\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "runs at load"],
  },
  {
    name: "alpha-eq-without-reads",
    breaks: "two trees alike but for their names are taken for one though a read of a name from outside differs: `[(x) => x, x]` and `[(y) => y, y]` one dictionary, the second's `y` read as the first's `x` (#1681)",
    file: "src/js/scopes.rs",
    find: "    a == b || (unnamed(a) == unnamed(b) && events(a) == events(b))\n",
    replace: "    a == b || unnamed(a) == unnamed(b)\n",
    unitTests: "js::scopes::tests::reads_from_outside_are_by_their_names",
  },
  {
    name: "alpha-eq-reads-of-any-declaration",
    breaks: "a read is taken to be of a declaration whichever it's of: `(x) => (x) => x` alike `(a) => (b) => a`, a closure's read of the outer parameter for the inner's (#1681)",
    file: "src/js/scopes.rs",
    find: "            Some(s) => Event::Read(self.tree.scopes[s].names.get(name).copied()),\n",
    replace: "            Some(_) => Event::Read(None),\n",
    unitTests: "js::scopes::tests::a_shadowed_read_is_of_the_innermost",
  },
  {
    name: "validate-global-unmarked",
    breaks: "a global's read, `console` of a `println!`, isn't taken for resolved by its name: every program refused, `console` read and declared nowhere",
    file: "src/js/scopes.rs",
    find: "            ExprKind::Var(name) => self.read(name.word(), Whose::of(name)),\n",
    replace: "            ExprKind::Var(name) => self.read(name.word(), Whose::Local),\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drop_past_a_panic.rs$"],
  },
  {
    name: "validate-cases-one-block",
    breaks: "a `switch`'s cases are taken for one scope, though each that declares is braced: two arms that bind one name are refused",
    file: "src/js/scopes.rs",
    find: "                    values.iter().for_each(|value| self.expr(value));\n                    self.block(body);\n",
    replace: "                    values.iter().for_each(|value| self.expr(value));\n                    self.block_in_scope(body);\n",
    unitTests: "js::validate::tests::a_block_declares_a_name_once",
  },
  {
    name: "validate-destructure-declares-nothing",
    breaks: "a destructuring's names aren't declared: a second of a name in one block let through, which JS refuses",
    file: "src/js/scopes.rs",
    find: "                StmtKind::Destructure { pattern, .. } => self.pattern_names(pattern),\n",
    replace: "                StmtKind::Destructure { .. } => {}\n",
    unitTests: "js::validate::tests::a_block_declares_a_name_once",
  },
  {
    name: "validate-method-binds-its-name",
    breaks: "a type's method is taken to bind its own name inside itself, which none does: a global of its name read there, React's `getComputedStyle`, is refused as shadowed (#1681)",
    file: "src/js/scopes.rs",
    find: "            js::Item::Namespace(namespace) => namespace.methods.iter().for_each(|m| self.function(m, false)),\n",
    replace: "            js::Item::Namespace(namespace) => namespace.methods.iter().for_each(|m| self.function(m, true)),\n",
    unitTests: "js::validate::tests::a_global_shadowed_is_refused",
  },
  {
    name: "locals-apart-of-no-module-read",
    breaks:
      "a local of a helper's word its scope reads, `copiedClone` beside std's `copiedClone(..)` read, keeps its word: JS reads the local, refused as `ModuleShadowed` (#1975)",
    file: "src/js/scopes.rs",
    find: "                let Some(of) = read.of.filter(|&of| of != 0 && read.whose == Whose::Module) else {\n",
    replace: "                let Some(of) = read.of.filter(|_| false) else {\n",
    tests: ["test/corpus.test.ts", "-t", "^(local_named_as_a_helper_its_scope_reads|clones_beside_a_generic_clone_of_a_copied_map).rs$"],
  },
  {
    name: "locals-apart-of-local-reads-too",
    breaks:
      "a read of a local's own variable is taken for a module name's, so every local read in its scope is respelled apart, `copiedClone$1` a name of no declaration's (#1975)",
    file: "src/js/scopes.rs",
    find: "                let Some(of) = read.of.filter(|&of| of != 0 && read.whose == Whose::Module) else {\n",
    replace: "                let Some(of) = read.of.filter(|&of| of != 0) else {\n",
    tests: ["test/corpus.test.ts", "-t", "^(local_named_as_a_helper_its_scope_reads|clones_beside_a_generic_clone_of_a_copied_map|locals_named_as_the_modules_names).rs$"],
    snapshots: true,
  },
];
