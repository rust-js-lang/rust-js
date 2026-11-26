// Mutations of crates/rust-js-lower/src/analysis/local_items.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "local-consts-none",
    breaks:
      "a `const` written in a function's body is its module's, beside the function",
    file: "crates/rust-js-lower/src/analysis/local_items.rs",
    find: "            within.then_some((def_id, (block.span, readers)))\n",
    replace:
      "            (within && false).then_some((def_id, (block.span, readers)))\n",
    tests: ["test/corpus.test.ts", "-t", "^local_consts.rs$"],
    snapshots: true,
  },
  {
    name: "local-const-read-outside",
    breaks:
      "a `const` written in a function's body that a method of the module's reads is the function's, which the method can't see",
    file: "crates/rust-js-lower/src/analysis/local_items.rs",
    find: "            within.then_some((def_id, (block.span, readers)))\n",
    replace: "            Some((def_id, (block.span, readers)))\n",
    tests: ["test/corpus.test.ts", "-t", "^local_consts.rs$"],
  },
  {
    name: "named-expression-any-nested",
    breaks:
      "a function a block makes but doesn't give is put where it's called, `(function loud(text) { .. })(text)`",
    file: "crates/rust-js-lower/src/analysis/local_items.rs",
    find: "    matches!(block.expr, Some(hir::Expr { kind: hir::ExprKind::Path(hir::QPath::Resolved(None, path)), .. })\n        if path.res == Res::Def(DefKind::Fn, def_id.to_def_id()))",
    replace: "    true",
    tests: [
      "test/jsx-children.test.ts",
      "-t",
      "a function a block makes and gives",
    ],
  },
  {
    name: "named-expression-recursive",
    breaks:
      "a function that calls itself is put where it's named, leaving its own call a hole",
    file: "crates/rust-js-lower/src/analysis/local_items.rs",
    find: "uses == 1 && given_by_its_block",
    replace: "uses >= 1 && given_by_its_block",
    tests: [
      "test/jsx-children.test.ts",
      "-t",
      "a function a block makes and gives",
    ],
  },
  {
    name: "named-expression-generic",
    breaks:
      "a generic function a block gives is put where it's named, without the dictionaries its caller gives",
    file: "crates/rust-js-lower/src/analysis/local_items.rs",
    find: " || tcx.generics_of(def_id).count() != 0",
    replace: "",
    tests: [
      "test/jsx-children.test.ts",
      "-t",
      "a function a block makes and gives",
    ],
  },
  {
    name: "local-functions-none",
    breaks:
      "a function written in a body is the module's, not its body's where it's written",
    file: "crates/rust-js-lower/src/analysis/local_items.rs",
    find: "                        hir::Node::Stmt(_)\n",
    replace: "                        hir::Node::Stmt(_) if false\n",
    tests: ["test/lowering-functions.test.ts", "-t", "local function there"],
  },
  {
    name: "local-function-static-user",
    breaks:
      "a function a static's initializer calls is written in its body, where the module's initializer can't call it",
    file: "crates/rust-js-lower/src/analysis/local_items.rs",
    find: "                    .any(|&user| user != def_id && user != parent(def_id) && !local.contains(&user))\n",
    replace:
      "                    .any(|&user| false && user != def_id && user != parent(def_id) && !local.contains(&user))\n",
    tests: ["test/lowering-functions.test.ts", "-t", "local function there"],
  },
  {
    name: "default-local-function-local",
    breaks:
      "a function written in a trait's default is declared in each impl's copy, where what it calls isn't imported, and the copy calls the trait module's, which isn't there",
    file: "crates/rust-js-lower/src/analysis/local_items.rs",
    find: "                    && tcx\n                        .trait_of_assoc(tcx.typeck_root_def_id(tcx.local_parent(local).to_def_id()))\n                        .is_none()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^default_inner_fn.rs$"],
  },
];
