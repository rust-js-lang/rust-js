// Mutations of src/lower/display.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "time-shown-as-a-number",
    breaks:
      "a clock's time or error, its nanoseconds in JS, is padded as a number, `{:>20?}` of a `SystemTimeError` its text as a whole, where std's gives the width to each field",
    file: "src/lower/display.rs",
    find: "    Num::of(ty).filter(|_| !own_rules)",
    replace: "    Num::of(ty)",
    tests: ["test/corpus.test.ts", "-t", "^(time_debug_as_unix|duration_debug_options)\\.rs$"],
  },
  {
    name: "unwrap-err-of-a-failed-write-unread",
    breaks: "`unwrap_err()` of a write that may fail is read as one that never does, and panics though it failed",
    file: "src/lower/display.rs",
    find: "            let result = match may_fail && may_be_write_error(&result) {",
    replace: "            let result = match false && may_be_write_error(&result) {",
    tests: ["test/corpus.test.ts", "-t", "^fmt_result_unwrap_err\\.rs$"],
  },
  {
    name: "unwrap-err-message-of-unwrap",
    breaks: "`unwrap_err()` of a write that didn't fail panics `unwrap()`'s message, not `unwrap_err()`'s",
    file: "src/lower/display.rs",
    find: '            let message = message.unwrap_or_else(|| Expr::str("called `Result::unwrap_err()` on an `Ok` value"));',
    replace: '            let message = message.unwrap_or_else(|| Expr::str("called `Result::unwrap()` on an `Ok` value"));',
    tests: ["test/corpus.test.ts", "-t", "^fmt_result_unwrap_err"],
  },
  {
    name: "held-write-error-taken-for-ok",
    breaks: "`is_ok()` of a `fmt::Result` held in a field or a `RefCell`, `held.0.is_ok()`, is `true`, only a variable's asked (#1011)",
    file: "src/lower/display.rs",
    find: "        _ => true,\n    }\n}\n",
    replace: "        js::ExprKind::Var(_) => true,\n        _ => false,\n    }\n}\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_error_as_a_value.rs$"],
  },
  {
    name: "format-pieces-last-only",
    breaks: "a format string of `concat!`'s pieces is its last piece alone",
    file: "src/lower/display.rs",
    find: '    if pieces.len() == 1 {\n        let (parts, lines) = pieces.pop().expect("a piece");',
    replace:
      '    if !pieces.is_empty() {\n        let (parts, lines) = pieces.pop().expect("a piece");',
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a format string of a concat! is its pieces joined by",
    ],
  },
  {
    name: "pretty-always-plain",
    breaks: "`{:#?}` of std's types is plain",
    file: "src/lower/display.rs",
    find: "            Pretty::Always => Some(true),\n",
    replace: "            Pretty::Always => Some(false),\n",
    tests: ["test/corpus.test.ts", "-t", "pretty_debug"],
  },
  {
    name: "run-time-options-ignored",
    breaks:
      "a number a `fmt` hands its `Formatter` on to ignores the options it's given at run time",
    file: "src/lower/display.rs",
    find: "        if !self.format_options() {\n            return None;\n        }",
    replace: "        if true {\n            return None;\n        }",
    tests: ["test/corpus.test.ts", "-t", "options_handed_on"],
  },
  {
    name: "run-time-options-everywhere",
    breaks:
      "a crate that gives no options but `{:#?}`'s applies them to each number its writers show: right, but not the JS a person writes",
    file: "src/lower/display.rs",
    find: "        if !self.format_options() {\n            return None;\n        }",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "pretty_debug"],
    snapshots: true,
  },
  {
    name: "run-time-float-precision-dropped",
    breaks:
      "an `f64` a `fmt` hands its `Formatter` on to ignores the precision it's given",
    file: "src/lower/display.rs",
    find: "                vec![value, options.clone(), Expr::var(show)],",
    replace: "                vec![value, Expr::js_undefined(), Expr::var(show)],",
    tests: ["test/corpus.test.ts", "-t", "options_handed_on"],
  },
  {
    name: "run-time-str-debug-padded",
    breaks:
      "a string's `{:?}` in a `fmt` given a width is padded, where std's isn't",
    file: "src/lower/display.rs",
    find: "            Std::FmtDisplay if self.is_string_like(ty) => value,",
    replace: "            _ if self.is_string_like(ty) => value,",
    tests: ["test/corpus.test.ts", "-t", "options_handed_on"],
  },
  {
    name: "options-dropped-allowed",
    breaks:
      "a width for a `&dyn Debug`, the string it shows already, is dropped",
    file: "src/lower/display.rs",
    find: "        if matches!(pretty, Pretty::Given(..))\n",
    replace: "        if false\n",
    tests: ["test/diagnostics.test.ts", "-t", "a width for a"],
  },
  {
    name: "std-display-dictionary-plain",
    breaks:
      "a generic `T`'s `{:>6}` of a number shows it unpadded: std's `Display` dictionary ignores the options",
    file: "src/lower/display.rs",
    find: '                Pretty::When(Expr::var("options")),\n            ),\n            false => (vec![param.as_str().into()], Pretty::Plain),\n        };\n        let shown = self.with_shown(shown_so, |cx| {\n',
    replace:
      '                Pretty::Plain,\n            ),\n            false => (vec![param.as_str().into()], Pretty::Plain),\n        };\n        let shown = self.with_shown(shown_so, |cx| {\n',
    tests: ["test/corpus.test.ts", "-t", "options_handed_on"],
  },
  {
    name: "fill-default-empty",
    breaks:
      "`f.fill()` of a placeholder that gives none is `''`, where it's a space",
    file: "src/lower/display.rs",
    find: 'Expr::bin(Op::Coalesce, member("fill"), Expr::str(" "))',
    replace: 'member("fill")',
    tests: ["test/corpus.test.ts", "-t", "formatter_queries"],
  },
  {
    name: "to-string-trait-assumed",
    breaks:
      "a `#![no_std]` crate's trait call crashes rust-js, looking up `alloc`'s `ToString`, which isn't loaded",
    file: "src/lower/display.rs",
    find: "        opt_std_item(self.tcx, StdItem::ToString)",
    replace: "        Some(std_item(self.tcx, StdItem::ToString))",
    tests: ["test/crates.test.ts", "-t", "no_std"],
  },
  {
    name: "std-radix-unprefixed",
    breaks: "`{:#x}` through a delegating impl shows no `0x`",
    file: "src/lower/display.rs",
    find: '                let prefix = Expr::cond(pretty.alternate(), Expr::str(radix.prefix()), Expr::str(""));',
    replace: '                let prefix = Expr::str("");',
    tests: ["test/corpus.test.ts", "-t", "user_fmt_traits"],
  },
  {
    name: "std-radix-unpadded",
    breaks: "`{:04x}` through a delegating impl isn't padded",
    file: "src/lower/display.rs",
    find: '            Pretty::When(options) if self.format_options() => Expr::call(\n                Expr::helper("$formatted"),\n                vec![text, options.clone(), Expr::bool(true)],\n',
    replace:
      '            Pretty::When(options) if false && self.format_options() => Expr::call(\n                Expr::helper("$formatted"),\n                vec![text, options.clone(), Expr::bool(true)],\n',
    tests: ["test/corpus.test.ts", "-t", "user_fmt_traits"],
  },
  {
    name: "failing-write-whole",
    breaks:
      "a `write!` of what fails writes nothing of its pieces before the one that fails",
    file: "src/lower/display.rs",
    find: "    let parts = match pieces {\n        true => printing::line_parts(written),",
    replace: "    let parts = match false {\n        true => printing::line_parts(written),",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "failing-call-uncaught",
    breaks:
      "a call that returns a `fmt::Result` and may fail isn't caught as its value",
    file: "src/lower/display.rs",
    find: "        if !self.is_write_result(output)\n            || self.formatter_param(def_id).is_some()",
    replace: "        if true\n            || self.formatter_param(def_id).is_some()",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "js-error-width-dropped",
    breaks:
      "`{:>9?}` of a `js::JsError` is its text with no width, not refused",
    file: "src/lower/display.rs",
    find: " || self.recognition().is_js_error(ty))\n",
    replace: ")\n",
    tests: ["test/diagnostics.test.ts", "-t", "a width for a JsError"],
  },
  {
    name: "string-call-unstringed",
    breaks:
      "`String(arg)` the program wrote is `${arg}` in a template, which throws on a symbol",
    file: "src/lower/display.rs",
    find: "        js::ExprKind::Stringed(value) => *value,\n",
    replace: "        js::ExprKind::Stringed(value) => *value,\n        js::ExprKind::Call(_, ref args) if args.len() == 1 => args[0].clone(),\n",
    tests: ["test/bindings.test.ts", "-t", "js::shown"],
  },
  {
    name: "stringed-not-string",
    breaks: "a number's `fmt` is `(value) => String(value)`, not `String`",
    file: "src/lower/display.rs",
    find: "            js::ExprKind::Stringed(value) => Some(&**value),\n",
    replace: "",
    tests: ["test/snapshots.test.ts", "-t", "queues: the generated JS"],
    snapshots: true,
  },
  {
    name: "fmt-result-err-is-ok",
    breaks: "`is_err()` of a `fmt::Result` that may be an `Err` is `false`",
    file: "src/lower/display.rs",
    find: "            FmtResultAnswer::Is(false) => Expr::bin(Op::Ne, result, Expr::js_undefined()),",
    replace: "            FmtResultAnswer::Is(false) => Expr::bool(false),",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "fmt-map-err-unmapped",
    breaks:
      "`map_err(f)` of a `fmt::Result` gives the error unmapped, not `f`'s",
    file: "src/lower/display.rs",
    find: "                let mapped = crate::js::result::err(self.applied(f, result));\n",
    replace: "                let mapped = crate::js::result::err(result);\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_result_map_err.rs$"],
  },
  {
    name: "fmt-result-map-err-kept-ok",
    breaks: "`map(f)` of a `fmt::Result` that's an `Err` is `Ok(f(()))`",
    file: "src/lower/display.rs",
    find: "            FmtResultAnswer::Map => Expr::cond(ok, constant, crate::js::result::err(result)),\n",
    replace: "            FmtResultAnswer::Map => constant,\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_result_map_err.rs$"],
  },
  {
    name: "fmt-result-map-unit-left-out",
    breaks:
      "`map(Ok)` of a `fmt::Result` calls `Ok` with nothing, written as an arrow called where it's made",
    file: "src/lower/display.rs",
    find: "                crate::js::result::ok(crate::lower::calls::apply(f, vec![Expr::js_undefined()]))\n",
    replace:
      "                crate::js::result::ok(crate::lower::calls::apply(f, Vec::new()))\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_result_map_err.rs$"],
    snapshots: true,
  },
  {
    name: "library-writer-takes-options",
    breaks:
      "a library's writer that takes no options is given a consumer's, where its dictionaries go",
    file: "src/lower/display.rs",
    find: "                let takes = self.krate.foreign.writers_take_options(writer.krate);\n",
    replace: "                let takes = self.writers_take_options();\n",
    tests: ["test/crates.test.ts", "-t", "as its own crate takes them"],
  },
  {
    name: "writer-takes-no-options-placeholder",
    breaks:
      "a writer that takes options, called where this one has none, isn't given their place: its dictionary is",
    file: "src/lower/display.rs",
    find: "            Some(_) => Some(self.writing.given.clone().unwrap_or_else(Expr::js_undefined)),\n",
    replace: "            Some(_) => self.writing.given.clone(),\n",
    tests: ["test/crates.test.ts", "-t", "as its own crate takes them"],
  },
  {
    name: "boxed-dyn-debug-raw",
    breaks:
      "a `Box<dyn Debug>` isn't seen as a `dyn Debug`, so one made a `&dyn Debug` is a pair of its pair (#1335; it was its value, interpolated raw, #174)",
    file: "src/lower/display.rs",
    find: "            && (ty.is_box() || self.is_rc(ty) && self.counted_rc(ty).is_none())\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^(boxed_dyn_debug|dyn_debug_shown_where_rust_calls_fmt)\\.rs$"],
    snapshots: true,
  },
  {
    name: "shown-text-taken-apart",
    breaks:
      "`${n}` given to `join` alone is taken apart, `n`, a number where a `String` is text",
    file: "src/lower/display.rs",
    find: "    if is_shown_text(&e) {\n        return vec![e];\n    }\n",
    replace: "",
    tests: ["test/bindings.test.ts", "-t", "js::shown's text is text"],
  },
  {
    name: "shown-texts-added",
    breaks:
      '`format!("{}{}", js::shown(a), js::shown(b))` is `${a}` + `${b}`, not one template, `${a}${b}`',
    file: "src/lower/display.rs",
    find: "    let strings_alone = !pieces.iter().any(|p| is_text(p) || is_shown_text(&p.0));\n",
    replace: "    let strings_alone = !pieces.iter().any(is_text);\n",
    tests: ["test/bindings.test.ts", "-t", "js::shown's text is text"],
  },
  {
    name: "separable-strings-added",
    breaks:
      "a `write!` of strings alone that fails part way is `s += a + b`, which loses what came before the one that fails",
    file: "src/lower/display.rs",
    find: "strings_alone && !(separable && pieces.len() > 1)",
    replace: "strings_alone",
    tests: ["test/corpus.test.ts", "-t", "^format_values_alone.rs$"],
  },
  {
    name: "instance-writer-options-library",
    breaks:
      "a writer a consumer compiles takes its Formatter's options as its library's do, not its crate's own, so `{:>9?}` of a `Label` isn't padded (ADRs 0137, 0914)",
    file: "src/lower/display.rs",
    find: "        match writer.is_local() || self.krate.foreign.is_instance(writer) {\n",
    replace: "        match writer.is_local() {\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "program's, alone or a dependency"],
  },
  {
    name: "class-writer-called-by-name",
    breaks:
      "a class's `toString` a writer calls with options is `Position.toString(p, options)`, `this` the constructor (#358)",
    file: "src/lower/display.rs",
    find: "        Ok(self.crate_called(writer, values))\n",
    replace: "        Ok(Expr::call(self.fn_ref(writer), values))\n",
    tests: ["test/corpus.test.ts", "-t", "^class_display.rs$"],
  },
  {
    name: "class-to-string-given-undefined",
    breaks:
      "a class's plain `{p}`, where writers take options, is `p.toString(undefined)` (#358)",
    file: "src/lower/display.rs",
    find: "        if self.takes_options(instance.def_id()) && !plain_to_string {\n",
    replace: "        if self.takes_options(instance.def_id()) {\n",
    tests: ["test/corpus.test.ts", "-t", "^class_display.rs$"],
    snapshots: true,
  },
  {
    name: "appended-text-unjoined",
    breaks:
      '`s.push_str(&js::concat_text(n))` is `s += "" + n`, its `String` not known text (ADR 0639, amended)',
    file: "src/lower/display.rs",
    find: "Expr::text_joined(target.clone(), part)",
    replace: "Expr::bin(Op::Add, target.clone(), part)",
    tests: ["test/bindings.test.ts", "-t", "any value is read as JS reads it"],
  },
  {
    name: "shown-value-unevaluated",
    breaks:
      "a value whose text doesn't read it, `()`'s `()`, a `Weak`'s `(Weak)`, an error's message, isn't made: the call that gives it never prints or panics (#1063)",
    file: "src/lower/display.rs",
    find: "        Some(value) if !shown.has_effects() => Expr::sequence(vec![value, shown]),\n",
    replace: "        Some(_) if !shown.has_effects() => shown,\n",
    tests: ["test/corpus.test.ts", "-t", "^text_of_a_value_made_by_a_call.rs$"],
  },
  {
    name: "formatted-unit-value-dropped",
    breaks:
      "a `()` shown given its `Formatter`'s options, `fmt::Debug::fmt(give(), f)`, isn't made: the call that gives it never runs (#1063)",
    file: "src/lower/display.rs",
    find: "            Std::FmtDebug if ty.is_unit() => evaluated_first(value.has_effects().then_some(value), Expr::str(\"()\")),\n",
    replace: "            Std::FmtDebug if ty.is_unit() => Expr::str(\"()\"),\n",
    tests: ["test/corpus.test.ts", "-t", "^text_of_a_value_made_by_a_call.rs$"],
  },
  {
    name: "fmt-by-a-given-bound-unasked",
    breaks:
      "formatting a value whose impl a bound the function is given covers, libm's `where Hexf<T>: fmt::LowerHex`, isn't its caller's dictionary's, so `fmt::LowerHex::fmt(self, f)` and `{:x}` of it are refused (#1115)",
    file: "src/lower/display.rs",
    find: "            || !self.has_user_impl(trait_id, ty) && self.has_evidence(ty::TraitRef::new(self.tcx, trait_id, [ty]))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fmt_trait_fmt_called_by_path.rs$"],
  },
  {
    name: "compiled-writer-shown-ignored",
    breaks:
      "a writer of a type a library's function shows with options takes them only where the crate's own writers do, so `Plain`'s `{:#?}` through its library is on one line (#1331)",
    file: "src/lower/display.rs",
    find: "            _ if self.krate.shown.of_writer(self.tcx, writer).any() => \"shown\",\n",
    replace: "            _ if false => \"shown\",\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_shows_a_consumers_value_with_options"],
  },
  {
    name: "shown-hex-unformatted",
    breaks:
      "a writer of a type a library's function shows in hex applies no placeholder's options, so its integers aren't hex, `Plain { n: 255 }` (#1331)",
    file: "src/lower/display.rs",
    find: "        self.krate.format_options || self.writing.shown.hex\n",
    replace: "        self.krate.format_options\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_shows_a_consumers_value_with_options"],
  },
  {
    name: "mir-writer-options-dropped",
    breaks: "MIR's writer takes no options, a `{:>6}` of it unpadded",
    file: "src/lower/display.rs",
    find: "        (formatter && self.compiled_writer_takes_options(writer)).then(|| self.fresh(Role::Options))\n",
    replace: "        (formatter && false).then(|| self.fresh(Role::Options))\n",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a writer from its MIR takes its Formatter's options",
    ],
  },
  {
    name: "valued-call-never-fails",
    breaks: "a call of `same<T>(t: T) -> T` given a write's result is taken never to fail, so its value is read as a write's that threw (#1410)",
    file: "src/lower/display.rs",
    find: "            || !(valued || self.call_may_fail(def_id, args))\n",
    replace: "            || !self.call_may_fail(def_id, args)\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_given_to_generic_code.rs$"],
  },
  {
    name: "valued-call-not-a-trait-methods",
    breaks: "a trait's method called through a dictionary isn't asked whether its write result is `Form::Valued`, so `?` of a value its entry gives goes on past its `Err` (#1634)",
    file: "src/lower/display.rs",
    find: "        let generic = self.is_rust_fn(def_id) || self.tcx.trait_of_assoc(def_id).is_some();\n",
    replace: "        let generic = self.is_rust_fn(def_id);\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_through_a_dictionary.rs$"],
  },
  {
    name: "valued-given-back-unchecked",
    breaks: "a write's result `same<T>(t: T) -> T` or a dictionary's `fn is(self) -> T` gives back is the error as a value where its caller's call throws it, so `same(r)?` of an `Err` goes on (#1410, #1634)",
    file: "src/lower/display.rs",
    find: "            true => Expr::call_helper(\"$writeCheck\", vec![value]),\n",
    replace: "            true => value,\n",
    tests: ["test/corpus.test.ts", "-t", "^(write_result_given_to_generic_code|write_result_through_a_dictionary).rs$"],
  },
  {
    name: "never-shown-of-enums-alone",
    breaks: "`!`, which 1.100's `Infallible` is, isn't taken as never shown, so a `Debug` of a `Result<_, Infallible>` writes what can't run (ADR 0161, ADR 0109 amended)",
    file: "src/lower/display.rs",
    find: "            ty::Never => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^infallible_shown_through_a_bound.rs$"],
  },
  {
    name: "binding-text-interpolated",
    breaks: "a binding's JS value is \\`${v}\\` again, whose `toString` a minifier takes to run nothing, moving a held argument past it (#1676)",
    file: "src/lower/display.rs",
    find: "        false => Expr::call_helper(\"$shown\", vec![value]),\n",
    replace: "        false => shown_text(value),\n",
    tests: ["test/effects.test.ts", "-t", "toString runs after every argument"],
  },
  {
    name: "plain-binding-text-called",
    breaks: "a number's `js::shown`, `${year}`, is `$shown(year)`, a call where its template runs nothing (#1676)",
    file: "src/lower/display.rs",
    find: "        true => shown_text(value),\n        false => Expr::call_helper(\"$shown\", vec![value]),\n",
    replace: "        true => Expr::call_helper(\"$shown\", vec![value]),\n        false => Expr::call_helper(\"$shown\", vec![value]),\n",
    tests: ["test/bindings.test.ts", "-t", "js::shown"],
  },
];
