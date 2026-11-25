// Mutations of crates/rust-js-mir/src/tidy/bindings/temporaries.rs (ADR 0569).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unread-named-for-what-it-holds",
    breaks:
      "a value Rust leaves `_`, `for _ in ids.borrow().iter()`'s, is named for what it holds, `const value = borrow[i]`, where it says nothing reads it, `const _`",
    file: "crates/rust-js-mir/src/tidy/bindings/temporaries.rs",
    find: "StmtKind::Const(name, value) if (is_temporary(name) && !name.is_unread()) || is_held_arg(name) => {\n",
    replace: "StmtKind::Const(name, value) if is_temporary(name) || is_held_arg(name) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_other.rs$"],
    snapshots: true,
  },
  {
    name: "held-prop-named-value",
    breaks:
      'what MIR holds for a prop is named `value`, not after the prop, `const href = made("h")`',
    file: "crates/rust-js-mir/src/tidy/bindings/temporaries.rs",
    find: "                        (Some(prop), _) => vec![prop.clone()],\n",
    replace: "                        (Some(_), _) => Vec::new(),\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "held-arg-named-value",
    breaks: "an argument MIR holds, `format!`'s, is named `value`, not `arg`",
    file: "crates/rust-js-mir/src/tidy/bindings/temporaries.rs",
    find: "                        true => Role::Arg,\n",
    replace: "                        true => Role::Value,\n",
    tests: ["test/lowering-strings.test.ts", "-t", "format!"],
  },
  {
    name: "mir-tidy-receiver-named-value",
    breaks:
      "MIR's trait object a method is called on is named `value`, not `receiver`",
    file: "crates/rust-js-mir/src/tidy/bindings/temporaries.rs",
    find: "                .or_insert_with(|| Role::Receiver.text().to_string());\n",
    replace: "                .or_insert_with(|| Role::Value.text().to_string());\n",
    tests: ["test/traits.test.ts", "-t", "MIR dictionaries are explicit"],
  },
  {
    name: "temporary-names-unsuggested",
    breaks:
      "a temporary is never named as rust-analyzer suggests, after the call that made it or the type it holds, but `value`",
    file: "crates/rust-js-mir/src/tidy/bindings/temporaries.rs",
    find: "                        (None, _) => suggested.get(name).cloned().unwrap_or_default(),\n",
    replace: "                        (None, _) => Vec::new(),\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_names.rs$"],
    snapshots: true,
  },
  {
    name: "held-arg-let-unnamed",
    breaks:
      "a held argument a `try` assigns, `let _arg; try { _arg = .. }`, keeps MIR's lowering's name, not `arg`",
    file: "crates/rust-js-mir/src/tidy/bindings/temporaries.rs",
    find: "                StmtKind::Let(name, _) if is_held_arg(name) => {\n                    renames.push((name.clone(), name.suggested(Vec::new(), Role::Arg.text())))\n                }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_drops_nothing.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-named-positional",
    breaks:
      "a temporary that reads a positional field is named after it, `const _0 = result._0;`, a temporary's name, where its type's, `tok`, says what it holds (run 40)",
    file: "crates/rust-js-mir/src/tidy/bindings/temporaries.rs",
    find: "                            if !spelled_as_temporary(field) =>\n",
    replace: "                            if true =>\n",
    tests: ["test/corpus.test.ts", "-t", "^match_clone_nested_results.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-not-named-by-its-call",
    breaks:
      "a temporary of a call's value, a `println!` argument's, is named by its role, `const value = RangePull.pullTwo(..)`, not its call's name, `pullTwo` (#890, #837)",
    file: "crates/rust-js-mir/src/tidy/bindings/temporaries.rs",
    find: "                        true => rust_js_lower::held::held_names(value),\n",
    replace: "                        true => Vec::new(),\n",
    tests: ["test/corpus.test.ts", "-t", "^(boxed_option_unwraps|option_iterator_of_generic_items|blanket_iterator_trait|interpreter).rs$"],
    snapshots: true,
  },
  {
    name: "copy-named-value",
    breaks: "a copy a temporary holds is named by its role, `const value$1 = marks.at == null ? ..`, not as what it copies, `at` (#1923)",
    file: "crates/rust-js-mir/src/tidy/bindings/temporaries.rs",
    find: "                                rust_js_lower::copies::copied_source(value).map(|source| &source.kind)\n",
    replace: "                                None::<&Expr>.map(|source| &source.kind)\n",
    tests: ["test/library-kept-copies.test.ts", "-t", "literal, new box and constant"],
  },
  {
    name: "guarded-read-named-by-the-program-inlined",
    breaks: "what a guard holds that the program names, `let Some(id) = *slot.borrow() else`, is read in place, `if (slot.value == null) .. const id = slot.value;`, its name lost (#2490)",
    file: "crates/rust-js-mir/src/tidy/bindings/temporaries.rs",
    find: "                        && suggested.get(name).is_none_or(Vec::is_empty) =>\n",
    replace: "                        && true =>\n",
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_taken_parts.rs$"],
    snapshots: true,
  },
  {
    name: "guarded-read-of-what-reads-otherwise",
    breaks: "what a guard holds, read through what doesn't read the same each time, a stored address's `$storedValue(cell, 4).value`, is read again at each use (#2490)",
    file: "crates/rust-js-mir/src/tidy/bindings/temporaries.rs",
    find: "                        && value.reads_same()\n                        && suggested",
    replace: "                        && suggested",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_nested_field_through_a_deref.rs$"],
    snapshots: true,
  },
];
