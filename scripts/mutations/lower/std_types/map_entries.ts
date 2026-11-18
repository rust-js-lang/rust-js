// Mutations of src/lower/std_types/map_entries.rs (ADR 0093): a map's
// entry, as tidy leaves it, the map's own steps (ADR 0059, amended).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-map-count-literal-key",
    breaks:
      '`*m.entry("a").or_insert(0) += 5` from MIR stays a handle, `$mutGet(m, "a")`',
    file: "src/lower/std_types/map_entries.rs",
    find: "js::ExprKind::Str(a), js::ExprKind::Str(b)) if a == b))",
    replace: "js::ExprKind::Str(a), js::ExprKind::Str(b)) if a == b && false))",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a count kept in a map from MIR",
    ],
  },
  {
    name: "inserted-entries-helper",
    breaks:
      "`m.entry(k).or_default().push(v)` stays `$orInsertWith(m, k, () => []).push(v)`, not `if (!m.has(k)) { m.set(k, []); } m.get(k).push(v);` (ADR 0059, amended)",
    file: "src/lower/std_types/map_entries.rs",
    find: "        let Some((map, key, made)) = made else {\n",
    replace:
      "        let Some((map, key, made)) = made.filter(|_| false) else {\n",
    tests: ["test/corpus.test.ts", "-t", "^entry_default_pushed.rs$"],
    snapshots: true,
  },
  {
    name: "inserted-entry-value-alone",
    breaks:
      "a count's `const current = $orInsert(m, k, 0)` is its map's steps too, `if (!m.has(k)) { .. } const current = m.get(k);`, three statements for one (ADR 0059, amended)",
    file: "src/lower/std_types/map_entries.rs",
    find: "            .filter(|e| !matches!(&e.kind, js::ExprKind::Call(callee, _) if callee.helper_name().is_some()))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^entry_default_pushed.rs$"],
    snapshots: true,
  },
];
