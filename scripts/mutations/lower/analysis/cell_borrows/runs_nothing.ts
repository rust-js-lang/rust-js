// Mutations of src/lower/analysis/cell_borrows/runs_nothing.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "untagged-from-runs-code",
    breaks:
      "a `From` into an untagged enum is taken to run code rust-js can't see, so a borrow held across webapi's `BlobPart::from(chunk)` is counted, `$borrow(buffer, true)` and a `try`/`finally` (rd535)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "        || crate::lower::untagged::from_gives_its_argument(recognition.foreign, id)\n",
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
    find: "        || given_what_runs_nothing(recognition, def_id, args, given)\n",
    replace: "",
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
  {
    name: "none-written-not-read",
    breaks: "a `runs_only_given` builtin given `None` where its options' type is an object is still counted, so the vercel-ui SDK's memo over `Intl.Locale(tag, None)` keeps `$withBorrowMut` (#2082)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "        .all(|(at, &input)| value_runs_nothing(recognition, input) || written_none(at))\n",
    replace: "        .all(|(at, &input)| value_runs_nothing(recognition, input) || (false && written_none(at)))\n",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "of no options"],
  },
  {
    name: "none-written-of-any-option",
    breaks: "an `Option` written `Some(options)`, a JS caller's object, is taken for `None`, so a builtin given it isn't counted where its getters may re-enter the borrow (#2082)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "            && adt.variant_index.as_u32() == 0\n            && adt.fields.is_empty())\n",
    replace: ")\n",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a JS caller's options"],
  },
  {
    name: "optional-call-counted",
    breaks: "`js::optional_call((tag, Locale::get_week_info), ())`, `tag.getWeekInfo?.()`, is counted though its method runs nothing, so the vercel-ui SDK's memo keeps `$withBorrowMut` (#2082)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "        || optional_call_runs_nothing(recognition, def_id, args)\n",
    replace: "",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "of a locale it made optionally"],
  },
  {
    name: "optional-call-unchecked-trusted",
    breaks: "`optional_call_unchecked`, whose value is asserted of another contract, is taken to run what its path's binding runs, though its method may not be the binding's (#2082)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "JsForm::OptionalCall { unchecked: false }",
    replace: "JsForm::OptionalCall { .. }",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "optionally, unchecked"],
  },
  {
    name: "cast-to-js-value-not-traced",
    breaks: "a read through `js::unknown(x)` is taken for a read of a JS value, not of `x`'s own type, so the program's struct cast is counted (#2106)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "                    && matches!(js_form(tcx, callee), JsForm::This)\n",
    replace: "                    && false\n",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a struct only the crate constructs"],
  },
  {
    name: "struct-js-reach-ignored",
    breaks: "a property read of the program's own struct runs nothing though JS may give it, so a JS caller's `Chunk`, whose `reason` getter notes again, passes the held borrow where Rust panics (#2109)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "pointers.js_may_make(adt.did()) || pointers.js_may_give(adt.did())",
    replace: "false",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a JS caller gives"],
  },
  {
    name: "struct-crate-built-counted",
    breaks: "a property read of a struct only the crate constructs is counted, against ADR 0262's difference 60, so its borrow is checked for nothing (#2109)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "pointers.js_may_make(adt.did()) || pointers.js_may_give(adt.did())",
    replace: "true",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a struct only the crate constructs"],
  },
  {
    name: "rc-not-peeled",
    breaks: "a read through an `Rc` is taken for std's `Rc`'s, so a heap of a JS caller's tasks reads `sortIndex` plainly under its queue's borrow (#2109)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: " || recognition.is_std_type(ty, StdItem::Rc)",
    replace: "",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a task a JS caller gives"],
  },
  {
    name: "generic-read-not-deferred",
    breaks: "a property read of a type parameter's value is counted outright, not settled by its caller, so the scheduler's heap of tasks only the crate constructs gets `$withBorrowMut` (#2109)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "    crate::lower::generic_facts::may_be_any_type(read).then(",
    replace: "    (crate::lower::generic_facts::may_be_any_type(read) && false).then(",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a task only the crate constructs"],
  },
  {
    name: "receiver-taken-at-its-word",
    breaks: "a binding's method is taken at its word of any receiver, so a JS caller's `Intl.Locale` subclass whose `maximize` calls back into the memo passes its held borrow where Rust panics (#2111)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "        && receiver.runs_its_own(recognition, (def_id, args)))",
    replace: "        && true)",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a JS caller's locale's method keeps"],
  },
  {
    name: "receiver-at-hand-unread",
    breaks: "a binding's method is counted of a receiver a builtin made too, so a memo that makes its locale and maximizes it through a private function gets `$withBorrowMut` (#2111)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "            (!crate::lower::generic_facts::may_be_any_type(read) && !read_may_run(recognition, given.pointers, read))\n                || given.at_hand.holds(recognition, given.thir, receiver)",
    replace: "            !crate::lower::generic_facts::may_be_any_type(read) && !read_may_run(recognition, given.pointers, read)",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "through a private function"],
  },
  {
    name: "field-read-at-hand-unread",
    breaks: "a field read of what's at hand is counted, so a locale the memo made, cast to the program's struct, gets `$withBorrowMut` (#2113)",
    file: "src/lower/analysis/cell_borrows/runs_nothing.rs",
    find: "    (read_may_run(recognition, given.pointers, read) && !given.at_hand.holds(recognition, given.thir, lhs))",
    replace: "    read_may_run(recognition, given.pointers, read)",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a locale the memo made, cast"],
  },
];
