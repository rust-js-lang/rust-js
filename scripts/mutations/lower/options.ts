// Mutations of src/lower/options.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "some-literal-boxed",
    breaks: "`Some(Some(4))` is `$some(4)`: right, but not the JS a person writes",
    file: "src/lower/options.rs",
    find: "        if (value.is_constant() && !nullish) || literal {",
    replace: "        if false && ((value.is_constant() && !nullish) || literal) {",
    tests: ["test/corpus.test.ts", "-t", "nested_options"],
    snapshots: true,
  },
  {
    name: "some-undefined-folded",
    breaks: "`Some(None)` is `undefined`, folded as a literal, and reads as `None`",
    file: "src/lower/options.rs",
    find: "        if (value.is_constant() && !nullish) || literal {",
    replace: "        if value.is_constant() || literal {",
    tests: ["test/corpus.test.ts", "-t", "nested_options"],
  },
  {
    name: "unwrap-untyped-debug",
    breaks: "`unwrap()` of an `Err` of an enum shows `{ TAG: \"Missing\", .. }`, not `Missing { .. }`",
    file: "src/lower/options.rs",
    find: "        let peeled = ty.peel_refs();\n        if peeled.is_integral()",
    replace: "        let peeled = ty.peel_refs();\n        if true || peeled.is_integral()",
    tests: ["test/corpus.test.ts", "-t", "unwrap_typed_debug"],
  },
  {
    name: "unwrap-debug-wrapped",
    breaks: "`unwrap()`'s debug is `(e) => stockErrorDebug_fmt(e)`, not the function itself",
    file: "src/lower/options.rs",
    find: "                if matches!(callee.kind, js::ExprKind::Var(_))",
    replace: "                if false && matches!(callee.kind, js::ExprKind::Var(_))",
    tests: ["test/corpus.test.ts", "-t", "unwrap_typed_debug"],
    snapshots: true,
  },
];
