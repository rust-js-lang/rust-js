// Mutations of src/format.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "transform-map-unmoved",
    breaks: "a transform's text is written with the map of the text it was given, which points at other lines",
    file: "src/format.rs",
    find: "    let map = moved(code, &text, map, &before, &after, js_file_name);\n    Ok((text, map))",
    replace: "    let _ = (&before, &after);\n    Ok((text, map.to_json_string()))",
    tests: ["test/settings.test.ts", "-t", "a transform's text is written"],
  },
  {
    name: "transform-layout-is-program",
    breaks: "Prettier's JSX text lines and `{\" \"}` count as another program, and its layout is refused",
    file: "src/format.rs",
    find: "        if !(space || spaces || kind == AstType::JSXText) {",
    replace: "        if true {",
    tests: ["test/settings.test.ts", "-t", "Prettier's layout"],
  },
  {
    name: "transform-unchecked",
    breaks: "a transform that takes out a statement is written, its map pointing at other code",
    file: "src/format.rs",
    find: "    if let Some(at) = (0..was.len().max(is.len())).find(|&i| was.get(i) != is.get(i)) {",
    replace: "    if let Some(at) = (0..0).find(|&i| was.get(i) != is.get(i)) {",
    tests: ["test/settings.test.ts", "-t", "is refused, and its input kept"],
  },
  {
    name: "format-moved-node-uncarried",
    breaks: "a node a layout moves onto a line of its own, `setCount(`, keeps no mapping, its line pointing nowhere in the Rust",
    file: "src/format.rs",
    find: "            && from != to\n",
    replace: "            && false\n",
    tests: ["test/settings.test.ts", "-t", "Prettier's layout"],
  },
  {
    "name": "format-pairs-layout",
    "breaks": "a `{\" \"}` the formatter made text is paired with a later node, and every mapping after it points a line early",
    "file": "src/format.rs",
    "find": "    let pairs = pair(&layout_free(before, code), &layout_free(after, text));",
    "replace": "    let pairs = pair(before, after);",
    "tests": [
      "test/jsx.test.ts",
      "-t",
      "keeps its mappings in order"
    ]
  },
  {
    name: "transform-meaning-unchecked",
    breaks: "a transform that renames a variable, or changes a text, of the same nodes is kept, another program",
    file: "src/format.rs",
    find: "            && a != b\n",
    replace: "            && false\n",
    tests: ["test/settings.test.ts", "-t", "changes the program"],
  },
  {
    name: "blank-lines-none",
    breaks: "top-level statements that span lines, or functions, aren't set apart by a blank line",
    file: "src/format.rs",
    find: "            (apart && !imports && last + 1 == next).then_some(next)\n",
    replace: "            (false && apart && !imports && last + 1 == next).then_some(next)\n",
    tests: ["test/corpus.test.ts", "-t", "static_fns"],
    snapshots: true,
  },
  {
    name: "blank-lines-in-imports",
    breaks: "an import that spans lines is set apart from the others, splitting the import block",
    file: "src/format.rs",
    find: "            (apart && !imports && last + 1 == next).then_some(next)\n",
    replace: "            (apart && last + 1 == next).then_some(next)\n",
    tests: ["test/snapshots.test.ts", "-t", "apis"],
    snapshots: true,
  },
  {
    name: "blank-lines-one-line",
    breaks: "two one-line statements are set apart, `const a = 1;` from `const b = 2;`",
    file: "src/format.rs",
    find: "|| first < last || next < next_last;\n",
    replace: "|| true;\n",
    tests: ["test/corpus.test.ts", "-t", "statics"],
    snapshots: true,
  },
  {
    name: "formatted-map-unmoved",
    breaks: "JS laid out by the formatter is written with the map of the JS before it, so its positions point at other lines and columns of the Rust",
    file: "src/format.rs",
    find: "    let (code, map) = formatted(&code, &unformatted, js_file_name, &options).unwrap_or((code, map));\n",
    replace: "    let (code, map) = formatted(&code, &unformatted, js_file_name, &options)\n        .map(|(formatted, _)| (formatted, map.clone()))\n        .unwrap_or((code, map));\n",
    tests: ["test/traits.test.ts", "test/format.test.ts", "-t", "trait method expressions keep their Rust source locations|maps handlers to the formatted source"],
  },
];
