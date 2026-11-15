// Mutations of src/source_format.rs, `rust-js fmt` (#1901, ADR 1166).
import type { Mutation } from "../mutations";

const file = "src/source_format.rs";
const one = ["test/format.test.ts", "-t", "rust-js fmt is the one formatter"];

export const mutations: Mutation[] = [
  {
    name: "fmt-jsx-bodies-as-rustfmt-moved",
    breaks: "a `jsx!` body is as rustfmt moved it, a multi-line string's second line with it, `{\"first\\n     second\"}`, which changes the program",
    file,
    find: "        let rust = kept_jsx_bodies(&text, &rust, &source.path)?;\n",
    replace: "",
    tests: one,
  },
  {
    name: "fmt-stdin-jsx-bodies-as-rustfmt-moved",
    breaks: "`rust-js fmt --stdin` gives a `jsx!` body as rustfmt moved it, a multi-line string's line with it",
    file,
    find: "    let rust = kept_jsx_bodies(source, &rust, &job.name)?;\n",
    replace: "",
    tests: one,
  },
  {
    name: "fmt-check-writes",
    breaks: "`rust-js fmt --check` writes each file it would change, where it says what it would and writes nothing",
    file,
    find: "            Mode::Check => print!(\"{}\", diff(path, text, out)),\n",
    replace: "            Mode::Check => std::fs::write(path, out).map_err(|e| format!(\"{}: {e}\", path.display()))?,\n",
    tests: ["test/format.test.ts", "-t", "formatter check is read-only"],
  },
  {
    name: "fmt-check-passes-a-change",
    breaks: "`rust-js fmt --check` exits 0 where a file would change, so CI passes what isn't formatted",
    file,
    find: "    Ok(!check || changed.is_empty())\n",
    replace: "    Ok(true)\n",
    tests: one,
  },
  {
    name: "fmt-module-dir-its-parent",
    breaks: "a module `a.rs` declares, `mod b;`, is looked for beside it, not in `a/`, so `a/b.rs` is never formatted",
    file,
    find: "            false => parent.join(file.file_stem().unwrap_or_default()),\n",
    replace: "            false => parent.clone(),\n",
    tests: one,
  },
  {
    name: "fmt-foreign-jsx-kept",
    breaks: "another crate's `foreign::jsx!` is kept as written through rustfmt, as if it were the compiler's, so its opaque body moves further each run",
    file,
    find: "        let pathed = i >= 2 && tokens[i - 1].0 == T::Colon && tokens[i - 2].0 == T::Colon;\n",
    replace: "        let pathed = false;\n",
    tests: one,
  },
];
