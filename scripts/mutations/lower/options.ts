// Mutations of src/lower/options.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flatten-unboxed",
    breaks: "`Some(None).flatten()` is the box `Some(None)` is, not `None`",
    file: "src/lower/options.rs",
    find: "            Std::OptionFlatten => self.some_value(arg()),",
    replace: "            Std::OptionFlatten => arg(),",
    tests: ["test/corpus.test.ts", "-t", "nested_options"],
  },
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
  {
    name: "filter-map-unfused",
    breaks: "`excerpt.filter(..).map(..)` keeps the filter's `Option` in a `const`, which pulls what JSX reads before it out of the JSX",
    file: "src/lower/options.rs",
    find: "                    false => filtered(&option),\n",
    replace: "                    false => None,\n",
    tests: ["test/jsx.test.ts", "-t", "filtered child, in place"],
  },
  {
    name: "filter-unwrap-or-unfused",
    breaks: "`title.filter(..).unwrap_or(\"Error\")` is `(title != null && .. ? title : undefined) ?? \"Error\"`, not `title || 'Error'`'s shape",
    file: "src/lower/options.rs",
    find: "                } else if let Some((kept, value)) = filtered(&option) {\n",
    replace: "                } else if let Some((kept, value)) = None::<(Expr, Expr)> {\n",
    tests: ["test/jsx.test.ts", "-t", "filtered child, in place"],
  },
  {
    name: "map-field-spilled",
    breaks: "`p.title.map(|t| ..)` reads `p.title` into a `const t` first, where a field of a plain value is read as it is",
    file: "src/lower/options.rs",
    find: "                            js::ExprKind::Member(object, _) if matches!(&object.kind, js::ExprKind::Var(name) if self.plain_value(name)) => {",
    replace: "                            js::ExprKind::Member(object, _) if false && matches!(&object.kind, js::ExprKind::Var(name) if self.plain_value(name)) => {",
    tests: [
      "test/jsx.test.ts",
      "-t",
      "field of what never changes in place"
    ]
  },
  {
    name: "kept-text-default-coalesced",
    breaks: "text kept where it isn't empty, or a default, is a conditional and `??`, not `title || \"\"`",
    file: "src/lower/options.rs",
    find: "&& let Some(text) = text_or(&option)",
    replace: "&& let Some(text) = text_or(&option).filter(|_| false)",
    tests: ["test/compiler.test.ts", "-t", "text kept where it isn't empty"],
  },
  {
    name: "kept-array-or",
    breaks: "an array kept where it isn't empty, or a default, is `list || [1]`, which keeps an empty one, as JS's `[]` is truthy",
    file: "src/lower/options.rs",
    find: "} else if self.is_string_like(generic_args.type_at(0))",
    replace: "} else if true",
    tests: ["test/compiler.test.ts", "-t", "text kept where it isn't empty"],
  },
  {
    name: "text-or-chain-unread",
    breaks: "`a || ` kept text, or a default, is `(a || ..) ?? \"\"`, not one `||` chain",
    file: "src/lower/options.rs",
    find: "return Some(Expr::bin(Op::Or, (**first).clone(), text_or(rest)?));",
    replace: "return None;",
    tests: ["test/compiler.test.ts", "-t", "text kept where it isn't empty"],
  },
];
