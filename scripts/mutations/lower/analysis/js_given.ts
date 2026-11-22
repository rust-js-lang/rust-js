// Mutations of crates/rust-js-lower/src/analysis/js_given.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "js-given-copy-unrouted",
    breaks:
      "a call of a function JS calls that gives it `()` calls its own JS, which unboxes, so `relay(()).is_some()` is `false`",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "        if main.unboxed.is_disjoint(&nullish) && js.is_subset(&main.js) && !main.returns_js {",
    replace: "        if true {",
    tests: ["test/corpus.test.ts", "-t", "js_given_copies"],
  },
  {
    name: "js-given-facing-boxed",
    breaks:
      "a function JS calls takes its type parameters as its crate's Rust gives them, so `pick(undefined, true)` gives JS `{ $someNone: 0 }`",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "                .filter(|&id| js_facing(tcx, recognition.foreign, id))",
    replace:
      "                .filter(|&id| false && js_facing(tcx, recognition.foreign, id))",
    tests: ["test/compiler.test.ts", "-t", "an Option of a generic T"],
  },
  {
    name: "js-given-passed-on-boxed",
    breaks:
      "what JS gave, passed on to the module's own function, is taken as never nullish, so `relay`'s JS calls `wrapped`, which boxes, and `relay(undefined)` gives JS a box",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "        ty::Param(param) if params.js.contains(&param.index) => Given::Js,",
    replace:
      "        ty::Param(param) if params.js.contains(&param.index) => Given::Defined,",
    tests: ["test/compiler.test.ts", "-t", "an Option of a generic T"],
  },
  {
    name: "mir-initializer-asked-visibility",
    breaks:
      "MIR asks a `const { .. }` block's visibility, which rustc has none of",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "    matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn)\n        && tcx.visibility(id)",
    replace: "    tcx.visibility(id)",
    tests: ["test/corpus.test.ts", "-t", "const_block_values"],
  },
  {
    name: "instance-exported",
    breaks:
      "a library's function the crate compiles itself is exported, `export function createContext`, as if JS or another crate could call it (ADR 0914)",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "    !foreign.is_instance(id) && exported_function(tcx, id)\n",
    replace: "    exported_function(tcx, id)\n",
    tests: [
      "test/crates-compiled-here.test.ts",
      "-t",
      "Copy bound takes a copy only where one is given",
    ],
  },
  {
    name: "flattened-alike-read",
    breaks:
      "a namespace's method whose function reads the `Option` it makes, `made.is_none()`, is called through a value where its result is flattened, which reads `Some(None)` as `None` (rd311)",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "        && made_and_returned(recognition, (callee, p), true, &mut IndexSet::default()))\n",
    replace: "        && true)\n",
    tests: ["test/namespaces.test.ts", "-t", "refused unless its result is flattened"],
  },
  {
    name: "flattened-alike-callee-read",
    breaks:
      "a function that gives its `Option` of what it's given from a function that reads it, `check(f)`, is taken as one that only makes and returns it (rd311)",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "                                        && made_and_returned(recognition, (callee, q), into_slot, seen)\n",
    replace: "                                        && true\n",
    tests: ["test/namespaces.test.ts", "-t", "refused unless its result is flattened"],
  },
  {
    name: "flattened-read-otherwise",
    breaks:
      "a namespace's method through a value whose result `.is_some()` reads, not `.flatten()`, is taken as flattened, `Some(None)` read as `None` (rd311)",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "    reads.1 == 1\n        && body.basic_blocks",
    replace: "    reads.1 == 1\n        || body.basic_blocks",
    tests: ["test/namespaces.test.ts", "-t", "refused unless its result is flattened"],
  },
  {
    name: "flattened-by-name-unread",
    breaks:
      "a namespace's method by its name whose result is read whole, `.is_some()`, is taken for one flattened, its library's JS given what may look like `None` (rd324)",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "        && (destinations.iter()).all(|&d| read_by_flatten(recognition, body, d));\n",
    replace: "        ;\n",
    tests: ["test/namespaces.test.ts", "-t", "by its name is its library"],
  },
  {
    name: "returned-iterator-unrouted",
    breaks: "a JS-facing function giving back an `impl Iterator` isn't routed, so its own JS gives a range's `{ start, end }`, which `[...upto(4)]` can't spread (#1999)",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "                    && (!super::mutation::type_params(tcx, id).is_empty() || gives_js_iterator(tcx, recognition, id))\n",
    replace: "                    && !super::mutation::type_params(tcx, id).is_empty()\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "given back to JS"],
  },
  {
    name: "returned-iterator-not-js",
    breaks: "a JS-facing function's own JS gives back what its `impl Iterator` reveals, not the JS iterator a JS caller steps (#1999)",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "                returns_js: returns_iterator(tcx, id),\n",
    replace: "                returns_js: false,\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "given back to JS"],
  },
  {
    name: "returned-iterator-no-rust-copy",
    breaks: "the crate's own Rust calls a JS-facing function's own JS, which gives back the JS iterator, where it reads what the `impl Iterator` reveals: no `uptoRevealed` (#1999)",
    file: "crates/rust-js-lower/src/analysis/js_given.rs",
    find: "        if main.unboxed.is_disjoint(&nullish) && js.is_subset(&main.js) && !main.returns_js {\n",
    replace: "        if main.unboxed.is_disjoint(&nullish) && js.is_subset(&main.js) {\n",
    tests: ["test/lowering-iterators.test.ts", "-t", "given back to JS"],
  },
];
