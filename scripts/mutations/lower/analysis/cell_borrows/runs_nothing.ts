// Mutations of src/lower/analysis/cell_borrows/runs_nothing.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "untagged-from-runs-code",
    breaks:
      "a `From` into an untagged enum is taken to run code rust-js can't see, so a borrow held across webapi's `BlobPart::from(chunk)` is counted, `$borrow(buffer, true)` and a `try`/`finally` (rd535)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "        || crate::lower::untagged::is_untagged_from(tcx, id)\n",
    replace: "",
    tests: ["test/webapi-objects.test.ts", "-t", "From into an untagged"],
  },
  {
    name: "jsx-builders-ask",
    breaks:
      "making an element counts as code that may ask, so a field's `RefCell` lent to a prop is a counted cell",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "            JsForm::Jsx(_)\n                | JsForm::Prop(_)",
    replace: "            JsForm::Text\n                | JsForm::Prop(_)",
    tests: [
      "test/jsx-evaluation-order.test.ts",
      "-t",
      "borrowed for a prop is its value",
    ],
  },
  {
    name: "binding-runs-nothing-ignored",
    breaks:
      "a binding marked `#[rust_js::runs_nothing]` may run anything, so a borrow held across it is counted, `$borrow(log.quiet)` (ADR 0867)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "        ) || runs_nothing(tcx, id))\n",
    replace: "        ))\n",
    tests: ["test/lowering-cells.test.ts", "-t", "unless it runs nothing"],
  },
  {
    name: "binding-runs-nothing-everywhere",
    breaks:
      "every binding runs nothing, so a borrow held while JS calls the crate back isn't counted and doesn't panic `already borrowed` (ADR 0867)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "        ) || runs_nothing(tcx, id))\n",
    replace: "        ) || true)\n",
    tests: ["test/lowering-cells.test.ts", "-t", "unless it runs nothing"],
  },
  {
    name: "is-defined-runs-code",
    breaks:
      "`js::is_defined`'s `typeof` may run anything, so React's `aggregateErrors` makes `thrownErrors` a counted cell (ADR 0867)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "                | JsForm::StrictEquals\n                | JsForm::Asks(_)\n",
    replace: "                | JsForm::StrictEquals\n",
    tests: ["test/js-errors.test.ts", "-t", "AggregateError"],
  },
  {
    name: "js-object-deref-runs-code",
    breaks:
      "an `&AggregateError`'s `Deref` to `&JsError` may run anything, so `thrownErrors` is a counted cell (ADR 0867)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "        || is_js_object_deref(tcx, tcx.parent(id))\n",
    replace: "",
    tests: [
      "test/js-builtins.test.ts",
      "test/js-errors.test.ts",
      "-t",
      "AggregateError|a Map is a WeakMap",
    ],
  },
  {
    name: "runs-only-given-ignored",
    breaks:
      "a binding that runs only what it's given, `console.error(..)` of text, may run anything, so a borrow held across it is counted, `$borrowMut(root.expirationTimes, true)`",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "    binding_runs_nothing(recognition.tcx, def_id) || given_what_runs_nothing(recognition, def_id, args)\n",
    replace: "    binding_runs_nothing(recognition.tcx, def_id)\n",
    tests: ["test/lowering-cell-borrows.test.ts"],
  },
  {
    name: "resolve-given-objects-runs-nothing",
    breaks:
      "a promise's resolve given the crate's object, whose `then` JS reads there and then, runs nothing, so a borrow held across it isn't counted (rd654)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "        && (args.type_at(1).tuple_fields().iter()).all(|input| value_runs_nothing(recognition, input))\n",
    replace: "        && (args.type_at(1).tuple_fields().iter()).all(|_| true)\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "promise's resolve is given text"],
  },
  {
    name: "given-objects-run-nothing",
    breaks:
      "an object given to `console.error(..)`, whose `toString` may borrow the cell, runs nothing, so a borrow held across it isn't counted and never panics `already borrowed`",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "            value_runs_nothing(recognition, items.type_at(0))\n        }\n        _ => false,\n",
    replace: "            value_runs_nothing(recognition, items.type_at(0))\n        }\n        _ => true,\n",
    tests: ["test/lowering-cell-borrows.test.ts"],
  },
  {
    name: "null-binding-runs-code",
    breaks:
      "`js::null()` made while a borrow of a field is held is taken to run code that may borrow it, so React's update clone's `gesture` is a checked `$fieldCell` everywhere",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "                | JsForm::Null\n",
    replace: "",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "js::null.. made while a borrow",
    ],
  },
  {
    name: "with-this-runs-its-closure",
    breaks:
      "`function::with_this(closure)` is said to run its closure, so a `pub` thread-local made of a factory of constructors whose constructor calls a fn pointer, React's `export const SyntheticEvent = createSyntheticEvent(EventInterface)`, is refused",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "                | JsForm::WithThis\n",
    replace: "",
    tests: [
      "test/js-constructors.test.ts",
      "-t",
      "a pub thread-local is made of a factory of constructors",
    ],
  },
  {
    name: "resolve-runs-anything",
    breaks:
      "a promise's resolve given text may run anything, so react-markup's `resolve(buffer.borrow().clone())` counts the borrow, `$borrow(buffer, true)`, refused where a library borrows a `RefCell<String>` unchecked (rd654)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "        && super::resolvers::is_resolve(tcx, thir, *callee)\n",
    replace: "        && super::resolvers::is_resolve(tcx, thir, *callee) && false\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "promise's resolve is given text"],
  },
  {
    name: "any-dyn-fn-a-resolve",
    breaks:
      "any `&dyn Fn(String)` a function is given is taken for a promise's resolve, so a borrow held across it isn't counted and never panics `already borrowed` where it borrows the cell (rd654)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "        && super::resolvers::is_resolve(tcx, thir, *callee)\n",
    replace: "        && (super::resolvers::is_resolve(tcx, thir, *callee) || true)\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "promise's resolve is given text"],
  },
];
