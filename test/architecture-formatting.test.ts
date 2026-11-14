// The formatting rules of the architecture, split from architecture.test.ts
// at its cap (a test file is at most 1500 lines): which type a formatting
// impl is of (#1798).
import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

const read = (path: string) => readFileSync(join(root, path), "utf8");
// In name order, as macOS lists them and Linux, CI's, doesn't.
const files = (directory: string): string[] => readdirSync(join(root, directory), { withFileTypes: true })
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  .flatMap(entry => entry.isDirectory() ? files(`${directory}/${entry.name}`) : [`${directory}/${entry.name}`]);
const sources = files("src").filter(path => path.endsWith(".rs"));

// The type a formatting impl is of is `recognition::formatted_adt`'s
// alone: its self type through the references, `Box`es and `Pin`s a crate
// may write a formatting impl for, `impl Display for &ByRef`'s `ByRef`.
// Four places took it as the self type's `ty_adt_def()`, each its own way,
// and none saw through a reference or a `Box`, so the dictionary carried no
// `fmtInto`, the writer wasn't framed and a library's contract didn't list
// the type (#1798). The owners of the writes into a writer a piece at a
// time ask it, and none reads an impl's self type's ADT itself.
test("the type a formatting impl is of is formatted_adt's alone", () => {
  const defines = /\bfn formatted_adt(_of)?\b/;
  expect(sources.filter(file => defines.test(read(file)))).toEqual(["src/lower/recognition/fmt.rs"]);
  const owners = [
    "src/lower/analysis/prints.rs",
    "src/lower/analysis/written_params.rs",
    "src/lower/library/written_into.rs",
    "src/lower/mir/std_calls/written_into.rs",
    ...files("src/lower/display").filter(path => path.endsWith(".rs")),
    "src/lower/display.rs",
  ];
  const shapes = [
    // An impl's self type's ADT: `tcx.type_of(imp)..ty_adt_def()`.
    /type_of\(imp\)[^;]{0,160}?ty_adt_def\(\)/g,
    /type_of\(imp\)[^;]{0,80}?ty::Adt\(/g,
    // A trait ref's self type's, the dictionary's.
    /tr\.self_ty\(\)\.(kind\(\)|ty_adt_def\(\))/g,
  ];
  const found = owners.flatMap(file => {
    const text = read(file);
    return shapes.flatMap(shape => [...text.matchAll(shape)].map(m => `${file}: ${m[0].split("\n")[0].trim()}`));
  });
  expect(found).toEqual([]);
});
