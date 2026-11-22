// Mutations of crates/rust-js-mir/src/std_calls/fmt_calls.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-radix-argument-unseen",
    breaks:
      "a `{:x}` argument from MIR isn't one `format_args!` shows by its spec",
    file: "crates/rust-js-mir/src/std_calls/fmt_calls.rs",
    find: "            Std::FmtDisplay | Std::FmtDebug | Std::FmtRadix(_) | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {\n",
    replace:
      "            Std::FmtDisplay | Std::FmtDebug | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {\n",
    tests: ["test/corpus.test.ts", "-t", "radix_negative"],
  },
  {
    name: "mir-user-write-refused",
    breaks:
      "`writeln!(out, ..)` to a writer of the crate's own from MIR is refused",
    file: "crates/rust-js-mir/src/std_calls/fmt_calls.rs",
    find: "            Std::UserWrite => lower(",
    replace: "            Std::UserWrite if false => lower(",
    tests: ["test/corpus.test.ts", "-t", "user_fmt_write"],
  },
  {
    name: "mir-format-failing-unmarked",
    breaks:
      "`format!` of a `Display` that fails throws a `fmt::Error`, not std's panic",
    file: "crates/rust-js-mir/src/std_calls/fmt_calls.rs",
    find: "                    if fails {\n                        return Ok(Value::Fmt(Std::FmtNew, output, joined));\n",
    replace:
      "                    if false && fails {\n                        return Ok(Value::Fmt(Std::FmtNew, output, joined));\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_format"],
  },
  {
    name: "format-arguments-in-place",
    breaks:
      "a format string's argument that does something is evaluated where the template formats it, after a `Debug` of an earlier one that prints",
    file: "crates/rust-js-mir/src/std_calls/fmt_calls.rs",
    find: "                    if this.formats_out_of_order(&items, &template, fails || prints, span)? {\n",
    replace:
      "                    if false && this.formats_out_of_order(&items, &template, fails || prints, span)? {\n",
    tests: ["test/corpus.test.ts", "-t", "^format_arguments_first.rs$"],
  },
  {
    name: "own-write-all-unwrapped",
    breaks:
      "std's `write_all` of the crate's writer that fails throws out of where its result is a value",
    file: "crates/rust-js-mir/src/std_calls/fmt_calls.rs",
    find: "                    true => this.fmt_try(vec![StmtKind::Return(Some(call)).at(js::Span::NONE)]),\n",
    replace: "                    true => call,\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "fmt-arg-through-refs",
    breaks:
      "a `&mut` or a `Box` shown is shown as itself, not what it points at, so `{:>6}` of a `Box<dyn Display>` pads its pointer's text, and a `&mut` is a handle read as one",
    file: "crates/rust-js-mir/src/std_calls/fmt_calls.rs",
    find: "                    _ => this.shown_through(value, ty, known),\n",
    replace: "                    _ => (value, ty),\n",
    tests: ["test/corpus.test.ts", "-t", "^(formatted_through_a_box_and_an_impl_display|boxed_mut_ref_shown).rs$"],
    snapshots: true,
  },
  {
    name: "fmt-as-str-of-formatted",
    breaks:
      "`as_str()` of arguments made with a value to format is `Some` of their text, so `format_args!(\"{count} items\").as_str()` is `Some(\"3 items\")` where native Rust's is `None`, and anyhow's `format_err` takes the literal's way (#1091)",
    file: "crates/rust-js-mir/src/std_calls/fmt_calls.rs",
    find: "                Some((false, _)) => lower(|_, _, _, _| Ok(Value::Expr(Expr::js_undefined()))),\n",
    replace: "                Some((false, _)) => lower(|_, _, values, _| Ok(values.next().expect(\"the arguments\"))),\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_arguments_as_str.rs$"],
  },
  {
    name: "fmt-as-str-of-elsewhere-taken",
    breaks: "`as_str()` of arguments made in another function, whose maker nothing here says, is `Some` of their text, `None` or not (#1091)",
    file: "crates/rust-js-mir/src/std_calls/fmt_calls.rs",
    find: "            Std::FmtAsStr => match this.arguments_made(_state.body, _state.at) {\n",
    replace: "            Std::FmtAsStr => match this.arguments_made(_state.body, _state.at).or(Some((true, rustc_middle::mir::RETURN_PLACE))) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "as_str\\(\\) of arguments made elsewhere"],
  },
  {
    name: "placeholder-dyn-debug-made-a-pair",
    breaks: "`{:#?}` of `&x as &dyn Debug`, shown where it's made, makes the pair and shows it through its `impl` (#1335)",
    file: "crates/rust-js-mir/src/std_calls/fmt_calls.rs",
    find: "                        Value::DynDebug(from, _, value) if known == Std::FmtDebug => {\n",
    replace: "                        Value::DynDebug(from, _, value) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^(dyn_debug_shown_where_rust_calls_fmt|pretty_debug)\\.rs$"],
    snapshots: true,
  },
];
