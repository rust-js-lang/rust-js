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
    find: '    js_root_fn(tcx, did).is_some_and(|name| name.as_str() == "truthy")\n',
    replace: '    js_root_fn(tcx, did).is_some_and(|name| name.as_str() == "truthy_")\n',
    tests: ["test/lowering-nullable-truthy.test.ts"],
  },
];
