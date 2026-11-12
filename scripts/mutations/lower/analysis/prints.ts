// Mutations of src/lower/analysis/prints.rs (ADR 1130).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "writer-that-prints-unseen",
    breaks: "a writer that prints as it formats is taken to print nothing, so `println!` builds its line whole and the print comes before it (#1490)",
    file: "src/lower/analysis/prints.rs",
    find: "                if printing.contains(&writer) || run.iter().any(|id| printing.contains(id)) {\n",
    replace: "                if false && (printing.contains(&writer) || run.iter().any(|id| printing.contains(id))) {\n",
    tests: ["test/corpus.test.ts", "-t", "^printed_while_a_line_is_formatted.rs$"],
  },
  {
    name: "what-runs-while-formatting-unseen",
    breaks: "a print a writer reaches through what it calls, a drop's, isn't written through stdout's line, so it lands before the line's text (#1490)",
    file: "src/lower/analysis/prints.rs",
    find: "                    found.while_formatting.extend(run);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^printed_while_a_line_is_formatted.rs$"],
  },
  {
    name: "writer-showing-a-printing-type-unseen",
    breaks: "a writer that shows a value whose writer prints, through std's `&dyn Debug`, isn't taken to print, and a derived `Debug` of it shows it out of order unrefused (#1495)",
    file: "src/lower/analysis/prints.rs",
    find: "        if more.is_empty() {\n",
    replace: "        if true || more.is_empty() {\n",
    tests: ["test/diagnostics.test.ts", "-t", "a derived Debug of what prints"],
  },
  {
    name: "composed-stopping-writer-unrefused",
    breaks: "a std value's `Debug` of a part whose writer may print and panic, where a `catch_unwind` may catch it, is let through, and the line loses what the parts before it wrote, `[!` where Rust keeps `[!D(1), !` (#1521)",
    file: "src/lower/analysis/prints.rs",
    find: "    if all_bodies.iter().any(|body| catches_unwinding(recognition, body)) {\n",
    replace: "    if false && all_bodies.iter().any(|body| catches_unwinding(recognition, body)) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "a Vec's Debug of what may print and panic"],
  },
  {
    name: "composed-stopping-writer-refused-unasked",
    breaks: "a std value's `Debug` of a part whose writer prints is refused in a crate that calls `catch_unwind` even where the writer never stops part-way (#1521)",
    file: "src/lower/analysis/prints.rs",
    find: "            .filter(|&writer| !never_unwinding.contains(writer) || failing.contains(writer))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^printing_writer_stopped_part_way_where_caught.rs$"],
  },
];
