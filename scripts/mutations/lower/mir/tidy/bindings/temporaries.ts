// Mutations of src/lower/mir/tidy/bindings/temporaries.rs (ADR 0569).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "temporary-takes-a-name-items-read",
    breaks:
      "a temporary beside a function written in the body takes a name of the module's that function reads, `const note = id(-1)`, and the function throws `note is not a function`",
    file: "src/lower/mir/tidy/bindings/temporaries.rs",
    find: "    taken.extend(items_read.iter().cloned());\n",
    replace: "    _ = items_read;\n",
    tests: ["test/corpus.test.ts", "-t", "^temporaries_beside_items_written_in_the_body.rs$"],
  },
  {
    name: "held-prop-named-value",
    breaks:
      'what MIR holds for a prop is named `value`, not after the prop, `const href = made("h")`',
    file: "src/lower/mir/tidy/bindings/temporaries.rs",
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
    file: "src/lower/mir/tidy/bindings/temporaries.rs",
    find: '                        true => "arg",\n',
    replace: '                        true => "value",\n',
    tests: ["test/lowering-strings.test.ts", "-t", "format!"],
  },
  {
    name: "mir-tidy-receiver-named-value",
    breaks:
      "MIR's trait object a method is called on is named `value`, not `receiver`",
    file: "src/lower/mir/tidy/bindings/temporaries.rs",
    find: '            given.entry(v.clone()).or_insert_with(|| "receiver".to_string());\n',
    replace: "",
    tests: ["test/traits.test.ts", "-t", "MIR dictionaries are explicit"],
  },
  {
    name: "temporary-names-unsuggested",
    breaks:
      "a temporary is never named as rust-analyzer suggests, after the call that made it or the type it holds, but `value`",
    file: "src/lower/mir/tidy/bindings/temporaries.rs",
    find: "                        (None, _) => suggested.get(name).cloned().unwrap_or_default(),\n",
    replace: "                        (None, _) => Vec::new(),\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_names.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-name-taken-suffixed",
    breaks:
      "a temporary whose suggested names are taken is given one apart, `point$1`, not `value`",
    file: "src/lower/mir/tidy/bindings/temporaries.rs",
    find: "            .find(|name| !taken.contains(name))\n",
    replace: "            .next()\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_names.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-takes-import-name",
    breaks:
      "a temporary is named after a function another module gives this one, so its import is renamed `prepare$1`",
    file: "src/lower/mir/tidy/bindings/temporaries.rs",
    find: "            taken.insert(symbol.export.clone());\n",
    replace: "            let _ = symbol;\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_names_imported.rs$"],
    snapshots: true,
  },
  {
    name: "temporaries-named-by-nesting",
    breaks:
      "temporaries are named outer lists first, so a `let` set late takes `next` from one read first",
    file: "src/lower/mir/tidy/bindings/temporaries.rs",
    find: "    renames.sort_by_key(|(from, ..)| first.get(from).copied().unwrap_or(usize::MAX));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^loop_drops.rs$"],
    snapshots: true,
  },
  {
    name: "held-arg-let-unnamed",
    breaks:
      "a held argument a `try` assigns, `let _arg; try { _arg = .. }`, keeps MIR's lowering's name, not `arg`",
    file: "src/lower/mir/tidy/bindings/temporaries.rs",
    find: '                StmtKind::Let(name, _) if is_held_arg(name) => renames.push((name.clone(), Vec::new(), "arg")),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_drops_nothing.rs$"],
    snapshots: true,
  },
  {
    name: "temporary-named-positional",
    breaks:
      "a temporary that reads a positional field is named after it, `const _0 = result._0;`, a temporary's name, where its type's, `tok`, says what it holds (run 40)",
    file: "src/lower/mir/tidy/bindings/temporaries.rs",
    find: "                            if !is_temporary(field) =>\n",
    replace: "                            if true =>\n",
    tests: ["test/corpus.test.ts", "-t", "^match_clone_nested_results.rs$"],
    snapshots: true,
  },
];
