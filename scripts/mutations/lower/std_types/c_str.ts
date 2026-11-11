// Mutations of src/lower/std_types/c_str.rs (ADR 1119).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "c-str-bytes-with-nul",
    breaks: "`c.to_bytes()` keeps the nul, `[104, 105, 0]` where Rust's is `[104, 105]`",
    file: "src/lower/std_types/c_str.rs",
    find: "        let bytes = |c: Expr| helper(\"$cstrBytes\", vec![c]);\n",
    replace: "        let bytes = |c: Expr| c;\n",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-str-count-with-nul",
    breaks: "`c.count_bytes()` counts the nul",
    file: "src/lower/std_types/c_str.rs",
    find: "CStrOp::Count => Expr::bin(js::Op::Sub, Expr::member(arg(), \"length\"), Expr::int(1)),",
    replace: "CStrOp::Count => Expr::member(arg(), \"length\"),",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-str-is-empty-of-no-bytes",
    breaks: "`c\"\".is_empty()` is `false`, its nul taken for a byte",
    file: "src/lower/std_types/c_str.rs",
    find: "CStrOp::IsEmpty => Expr::bin(js::Op::Eq, Expr::member(arg(), \"length\"), Expr::int(1)),",
    replace: "CStrOp::IsEmpty => Expr::bin(js::Op::Eq, Expr::member(arg(), \"length\"), Expr::int(0)),",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-string-new-unconverted",
    breaks: "`CString::new(\"text\")` is given the string, not its bytes",
    file: "src/lower/std_types/c_str.rs",
    find: "                let bytes = self.given_bytes(given, &call, out)?;\n",
    replace: "                let bytes = given;\n",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-string-new-of-text-as-std-values",
    breaks: "`CString::new(\"text\")`'s bytes are asked of `std_values`, which lowers no string's method",
    file: "src/lower/std_types/c_str.rs",
    find: "        if let Std::Text(op) = known {\n",
    replace: "        if let (false, Std::Text(op)) = (true, known) {\n",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-str-error-item-first",
    breaks: "`e.into_vec()` of a `NulError` is its position",
    file: "src/lower/std_types/c_str.rs",
    find: "CStrOp::Item(at) => Expr::index(arg(), Expr::int(at)),",
    replace: "CStrOp::Item(_) => Expr::index(arg(), Expr::int(0)),",
    tests: ["test/corpus.test.ts", "-t", "^c_strings.rs$"],
  },
  {
    name: "c-str-pointer-unsigned",
    breaks: "a C string's pointer reads its bytes as `u8`s, not as wasm32's `c_char`, so a byte above 127 isn't negative (#1291)",
    file: "src/lower/std_types/c_str.rs",
    find: "                self.recast(first, self.tcx.types.u8, char, call.span)?\n",
    replace: "                let _ = char;\n                first\n",
    tests: ["test/corpus.test.ts", "-t", "^c_string_pointers.rs$"],
  },
];
