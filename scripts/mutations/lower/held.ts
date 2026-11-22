// crates/rust-js-lower/src/held.rs's mutations (#2299 moved them with their code).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "place-where-made-handle-unfixed",
    breaks:
      "a handle on a `Cell` a field is, `node.0.v` borrowed before `node.0 = b`, isn't fixed where it's made, so `set` writes the new node's (#1034)",
    file: "crates/rust-js-lower/src/held.rs",
    find: "            js::ExprKind::Handle(on, storage) => Expr::stored_handle(self.place_where_made(*on, out), storage),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^reference_a_field_holds_set_again.rs$"],
  },
  {
    name: "subject-not-named-by-its-call",
    breaks: "an `Option` read twice is named after what reads it, `option` or a closure's parameter, `l`, not after the call that made it, `next` (#769)",
    file: "crates/rust-js-lower/src/held.rs",
    find: "            .filter(|name| !self.names.borrow().contains(*name) || self.locals.mir.subjects.contains(*name))\n",
    replace: "            .filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_drops.rs$"],
    snapshots: true,
  },
  {
    name: "subject-name-taken-from-the-program",
    breaks: "a subject named after its call takes a name the program's own has, `make$1` beside its `make` (#769)",
    file: "crates/rust-js-lower/src/held.rs",
    find: "            .filter(|name| !self.names.borrow().contains(*name) || self.locals.mir.subjects.contains(*name))\n",
    replace: "            .filter(|_| true)\n",
    tests: ["test/corpus.test.ts", "-t", "^option_combinator_drops.rs$"],
    snapshots: true,
  },
  {
    name: "pair-place-holder-unnamed",
    breaks: "what a pair's place is reached through, a reassigned `x` of `x.value`, isn't named where the pair's made, so the pair reads what `x` is later (#968)",
    file: "crates/rust-js-lower/src/held.rs",
    find: "            js::ExprKind::Var(name) if !self.alike_local(name) && !self.module_name(name) => {\n",
    replace: "            js::ExprKind::Var(name) if false && !self.alike_local(name) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_of_a_generic_impl_written_whole.rs$"],
  },
  {
    name: "kept-place-object-unnamed",
    breaks: "the object a kept place is reached through, `h.node` of `h.node.count`, isn't named where the borrow's made, so `h.node = other` after moves the handle to the other node (#988)",
    file: "crates/rust-js-lower/src/held.rs",
    find: "            js::ExprKind::Member(_, field) => {\n                let field = field.clone();\n                self.spill(Base::Named(&field), e, out)\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^kept_handle_of_a_place_set_again.rs$"],
  },
  {
    name: "fixed-place-facts-dropped",
    breaks:
      "a place made once, a cell's taken, forgets what's known of it, so `claimed ?? null`",
    file: "crates/rust-js-lower/src/held.rs",
    find: "        Expr { facts, ..fixed }\n",
    replace: "        Expr { facts: js::Facts::NONE, ..fixed }\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a nullable field's value a variable holds",
    ],
  },
  {
    name: "from-end-place-not-fixed",
    breaks:
      "an item counted from the end, given through a `&mut`, `zero(last)`, is `const at = v.length - 1` and `v[at]`, not its place, `v[v.length - 1]`",
    file: "crates/rust-js-lower/src/held.rs",
    find: "                && (index.is_constant() || matches!(index.kind, js::ExprKind::Var(_)) || from_end(object, index))\n",
    replace:
      "                && (index.is_constant() || matches!(index.kind, js::ExprKind::Var(_)))\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pattern_mut_rest.rs$"],
    snapshots: true,
  },
];
