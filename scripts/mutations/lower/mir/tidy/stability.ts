// Mutations of src/lower/mir/tidy/stability.rs: expression stability across calls.
import type {Mutation} from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-tidy-stable-literal-arguments-kept",
    breaks: "stable fresh object and array arguments prevent later native getter expressions from remaining at the call site",
    file: "src/lower/mir/tidy/stability.rs",
    find: "ExprKind::Jsx(_) | ExprKind::Object(..) | ExprKind::Array(_) => inert_element(e, set),",
    replace: "ExprKind::Jsx(_) => inert_element(e, set),",
    tests: ["test/lowering-native-aggregate-arguments.test.ts"],
  },
  {
    name: "mir-tidy-literal-contents-unchecked",
    breaks: "a literal's getters, computed keys, spreads or closure-written contents are treated as stable",
    file: "src/lower/mir/tidy/stability.rs",
    find: "ExprKind::Jsx(_) | ExprKind::Object(..) | ExprKind::Array(_) => inert_element(e, set),",
    replace: "ExprKind::Jsx(_) | ExprKind::Object(..) | ExprKind::Array(_) => true,",
    unitTests: "lower::mir::tidy::literal_movement_tests::fresh_literals_require_stable_contents",
  },
  {
    name: "mir-tidy-inlined-past-changed-reads",
    breaks:
      "a temporary that does something is written past reads of what it changes, `d.n + reset(d)`",
    file: "src/lower/mir/tidy/stability.rs",
    find: "    let plain_root = matches!(&root.kind, ExprKind::Var(v) if plain.contains(v.as_str()) && !set.contains(v.as_str()));\n",
    replace: "    let plain_root = true;\n",
    tests: ["test/corpus.test.ts", "-t", "assignment_order"],
  },
  {
    name: "mir-tidy-constant-objects-block",
    breaks:
      "a call isn't read in place past an object of constants before it, `const _3 = unquoted(..)`",
    file: "src/lower/mir/tidy/stability.rs",
    find: "        ExprKind::Jsx(_) | ExprKind::Object(..) | ExprKind::Array(_) => inert_element(e, set),\n",
    replace:
      "        ExprKind::Object(..) if e.is_made_of_constants() => false,\n        ExprKind::Jsx(_) | ExprKind::Object(..) | ExprKind::Array(_) => inert_element(e, set),\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "past an object of constants",
    ],
  },
  {
    name: "plain-field-blocks-inline",
    breaks:
      "a prop MIR holds for a borrow stays a `const _14`, read past `meta.tag` of a plain value",
    file: "src/lower/mir/tidy/stability.rs",
    find: "    let plain_root = matches!(&root.kind, ExprKind::Var(v) if plain.contains(v.as_str()) && !set.contains(v.as_str()));\n",
    replace: "    let plain_root = false;\n",
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
    name: "mir-tidy-import-blocks-inline",
    breaks:
      "a prop MIR holds for a borrow stays a `const _13`, read past an imported tag, `<Seo>`",
    file: "src/lower/mir/tidy/stability.rs",
    find: "        ExprKind::Symbol(_) => true,\n        ExprKind::Arrow(..)",
    replace: "        ExprKind::Arrow(..)",
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
    name: "mir-tidy-sibling-element-blocks",
    breaks:
      "a prop MIR holds for a borrow stays a `const _16`, read past a sibling element, `<Search isOpen={show} />`",
    file: "src/lower/mir/tidy/stability.rs",
    find: "        ExprKind::Jsx(_) | ExprKind::Object(..) | ExprKind::Array(_) => inert_element(e, set),\n",
    replace: "        ExprKind::Jsx(_) | ExprKind::Object(..) | ExprKind::Array(_) => !matches!(&e.kind, ExprKind::Jsx(_)) && inert_element(e, set),\n",
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
    name: "mir-tidy-closure-made-blocks",
    breaks:
      "a closure's making, `const value = () => ..`, blocks props held before it, `const value$3 = cn(..)`",
    file: "src/lower/mir/tidy/stability.rs",
    find: "        ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => true,\n        ExprKind::Member(of, _) => {\n",
    replace: "        ExprKind::Member(of, _) => {\n",
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
    name: "mir-tidy-choice-blocks",
    breaks:
      'a conditional of parameters, `selected ? "page" : undefined`, blocks a prop held after it',
    file: "src/lower/mir/tidy/stability.rs",
    find: "        ExprKind::Cond(a, b, c) => past(a) && past(b) && past(c),\n",
    replace: "",
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
    name: "inert-element-chosen-not",
    breaks:
      "a child chosen of elements, `open ? <b /> : <i />`, stops classes held for a later child being read in place",
    file: "src/lower/mir/tidy/stability.rs",
    find: "        ExprKind::Cond(a, b, c) => [a, b, c].iter().all(|e| inert_element(e, changing)),\n",
    replace: "",
    tests: ["test/snapshots.test.ts", "-t", "^apis"],
    snapshots: true,
  },
  {
    name: "spread-items-not-read-past-calls",
    breaks:
      "`...first` of an array whose items nothing changes stops a later call's value being read in place",
    file: "src/lower/mir/tidy/stability.rs",
    find: '            if !set.contains(v.as_str()) && plain.contains(&format!("...{v}"))),\n',
    replace: "            if !set.contains(v.as_str()) && false),\n",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "arrays joined are one array of their items",
    ],
  },
  {
    name: "mir-tidy-template-unread-past",
    breaks:
      "text made of what calls can't change, `${a} ${b}`, isn't read past a call, and what follows it stays a function called in place",
    file: "src/lower/mir/tidy/stability.rs",
    find: "        ExprKind::Template(_, values, _) => values.iter().all(past),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^format_args_order.rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-pure-call-unread-past",
    breaks:
      "a call of the runtime's that only reads, `$debugStr(s)`, isn't read past a call",
    file: "src/lower/mir/tidy/stability.rs",
    find: "        ExprKind::Call(callee, args) if callee.helper_name().is_some_and(crate::runtime::is_pure) => {\n            args.iter().all(past)\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^text_edits.rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-item-unread-past",
    breaks:
      "an item of a plain value, `pair[0]`, isn't read past a call, as its field is, and what follows stays a function called in place",
    file: "src/lower/mir/tidy/stability.rs",
    find: "        ExprKind::Index(_, index) => plain_root && past(index),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^const_generic_traits.rs$"],
    snapshots: true,
  },
  {
    name: "tidy-applied-call-not-past",
    breaks:
      "a function called at once on a held value is taken to read what a call may change, and the `Option` formatted after it stays a function called in place",
    file: "src/lower/mir/tidy/stability.rs",
    find: "            past(given) && read_past_calls(result, set, &plain)\n",
    replace: "            false\n",
    tests: ["test/corpus.test.ts", "-t", "^iter_next_back.rs$"],
    snapshots: true,
  },
  {
    name: "plain-iterator-spread-passes",
    breaks: "a spread of a plain value that's an iterator, a function's `inspect(see)`, is taken to do nothing, so `[...it, probe(9)]` runs `see` before `probe(9)` where Rust makes the chain's argument first (#1015)",
    file: "src/lower/mir/tidy/stability.rs",
    find: "            if !set.contains(v.as_str()) && plain.contains(&format!(\"...{v}\"))),\n",
    replace: "            if !set.contains(v.as_str()) && (plain.contains(v.as_str()) || plain.contains(&format!(\"...{v}\")))),\n",
    tests: ["test/corpus.test.ts", "-t", "^inspect_by_a_function_before_a_chain_argument.rs$"],
  },
  {
    name: "plain-field-of-a-struct-beside-a-cell-not-read-past-calls",
    breaks:
      "a struct literal of a struct with a `Cell` beside its plain fields hoists each value after a call into a temporary, `const value$2 = shouldSet(t) ? ..; return { .., text: value$2 }`, as its plain field `instance.id` isn't taken as unchanged by the call (#1045)",
    file: "src/lower/mir/tidy/stability.rs",
    find: "                || (plain_field(e, plain) && matches!(&root.kind, ExprKind::Var(v) if !set.contains(v.as_str())))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "literal_of_a_struct_beside_a_cell"],
    snapshots: true,
  },
  {
    name: "js-global-unknown-to-tidy",
    breaks: "a JS global lowering named, `Math` of `Math.max`, is taken to change past a call, so a read of it is held apart (#1674)",
    file: "src/lower/mir/tidy/stability.rs",
    find: "                    && (of.facts.has(js::Known::BuiltIn) || of.facts.has(js::Known::Lasting))",
    replace: "                    && of.facts.has(js::Known::Lasting)",
    tests: ["test/effects.test.ts", "-t", "held past a JS global's read"],
  },
];
