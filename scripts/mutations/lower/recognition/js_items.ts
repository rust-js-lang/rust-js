// Mutations of src/lower/recognition/js_items.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "untyped-class-never",
    breaks:
      "no `js::Class` is taken to be untyped, a class held, so its construction as any value names `js::new(class, ..)`, which rustc refuses of one",
    file: "src/lower/recognition/js_items.rs",
    find: "    is_js_class(tcx, class.did())\n        && args.types().next().is_some_and(|of| {\n",
    replace: "    false\n        && args.types().next().is_some_and(|of| {\n",
    tests: [
      "test/js-constructors.test.ts",
      "-t",
      "a constructor held as a value of any shape is refused",
    ],
  },
  {
    name: "js-truthy-unrecognized",
    breaks: "`js::truthy` isn't known for JS's `if (x)`, so it proves nothing of `x` (#1902)",
    file: "src/lower/recognition/js_items.rs",
    find: '    js_root_fn(tcx, did).is_some_and(|name| matches!(name.as_str(), "truthy" | "is_function"))\n',
    replace: '    js_root_fn(tcx, did).is_some_and(|name| matches!(name.as_str(), "truthy_" | "is_function"))\n',
    tests: ["test/lowering-nullable-truthy.test.ts"],
  },
  {
    name: "js-is-function-unrecognized",
    breaks: "`js::is_function`'s true edge proves nothing of what it tests, so React's scheduler's `task.callback = continuation` is tidy's answer of `continuation ?? null` (#1933)",
    file: "src/lower/recognition/js_items.rs",
    find: '    js_root_fn(tcx, did).is_some_and(|name| matches!(name.as_str(), "truthy" | "is_function"))\n',
    replace: '    js_root_fn(tcx, did).is_some_and(|name| matches!(name.as_str(), "truthy"))\n',
    tests: ["test/lowering-nullable-tests.test.ts", "-t", "written more than once"],
  },
  {
    name: "js-error-of-another-crate",
    breaks: "`js::JsError` isn't told apart, its `{:?}` refused",
    file: "src/lower/recognition/js_items.rs",
    find: 'tcx.crate_name(did.krate).as_str() == "js" && tcx.item_name(did).as_str() == "JsError"',
    replace: 'tcx.crate_name(did.krate).as_str() == "jsx" && tcx.item_name(did).as_str() == "JsError"',
    tests: [
      "test/js-errors.test.ts",
      "-t",
      "a JS error is shown as JS shows it",
    ],
  },
];
