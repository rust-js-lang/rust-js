// Mutations of src/lower/mir/tidy/mut_items.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

const tests = ["test/corpus.test.ts", "-t", "^iter_mut_index_loops.rs$"];

export const mutations: Mutation[] = [
  {
    name: "mut-items-handle-given",
    breaks: "a loop that gives its item's `&mut` on, `double(v)`, is an index loop too, which gives `double` nothing (#320)",
    file: "src/lower/mir/tidy/mut_items.rs",
    find: "        || js::mentions_in(body, &handle) != values\n",
    replace: "",
    tests,
  },
  {
    name: "mut-items-zip-longer",
    breaks: "a zip's index loop runs as long as the longer side, reading `chunk[3]` past its end (#320)",
    file: "src/lower/mir/tidy/mut_items.rs",
    find: '            let min = Expr::member(Expr::var("Math"), "min");\n',
    replace: '            let min = Expr::member(Expr::var("Math"), "max");\n',
    tests,
  },
  {
    name: "mut-items-enumerate-kept",
    breaks: "`for (i, cell) in row.iter_mut().enumerate()` stays `$mutItems(row).entries()` and `cell.value = i`, not `row[i] = i` (#354)",
    file: "src/lower/mir/tidy/mut_items.rs",
    find: "            if let Some(source) = crate::lower::iterators::enumeration_source(iterable)\n",
    replace: "            if false && let Some(source) = crate::lower::iterators::enumeration_source(iterable)\n",
    tests,
    snapshots: true,
  },
  {
    name: "mut-items-zip-kept",
    breaks: "`for (s, c) in state.iter_mut().zip(chunk)` stays `$lazyZip($iter($mutItems(state)), chunk.slice())` (#320)",
    file: "src/lower/mir/tidy/mut_items.rs",
    find: '            if callee.helper_name() == Some("$lazyZip")\n',
    replace: '            if false && callee.helper_name() == Some("$lazyZip")\n',
    tests,
    snapshots: true,
  },
];
