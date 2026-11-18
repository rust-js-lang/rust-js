// Mutations of src/lower/std_types/map_entries.rs (ADR 0093): a map's
// entry, as tidy leaves it, the map's own steps (ADR 0059, amended).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-map-count-literal-key",
    breaks:
      '`*m.entry("a").or_insert(0) += 5` from MIR stays a handle, `$mutGet(m, "a")`',
    file: "src/lower/std_types/map_entries.rs",
    find: "js::ExprKind::Str(a), js::ExprKind::Str(b)) if a == b)\n",
    replace: "js::ExprKind::Str(a), js::ExprKind::Str(b)) if a == b && false)\n",
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
    find: "        let Some((map, key, made)) = put_in(slot) else {\n",
    replace:
      "        let Some((map, key, made)) = put_in(slot).filter(|_| false) else {\n",
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
  {
    name: "entry-read-back-kept-helpers",
    breaks: "an entry's value put in as its statement alone, then read whole by the next, `*cache.entry(key).or_insert_with(|| resolve(key))`, stays `$orInsertWith(cache, key, () => ..); return $mutGet(cache, key).value;`, not the map's own steps, `if (!cache.has(key)) { cache.set(key, ..); } return cache.get(key);`, as the vercel-ui SDK's memo reads (G3)",
    file: "src/lower/std_types/map_entries.rs",
    find: "        if let Some((map, key, made)) = put\n",
    replace: "        if let Some((map, key, made)) = put.filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^entry_put_in_then_read\\.rs$"],
    snapshots: true,
  },
  {
    name: "entry-read-back-not-read-whole",
    breaks: "`first` doesn't stop at a `$mutGet(m, k).value` read, so the read the put-in statement's next does first isn't found, and the entry stays the helpers (G3)",
    file: "src/lower/std_types/map_entries.rs",
    find: "            js::ExprKind::Member(ref object, ref field) if field == js::HELD && got(object).is_some() => e,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^entry_put_in_then_read\\.rs$"],
    snapshots: true,
  },
];
