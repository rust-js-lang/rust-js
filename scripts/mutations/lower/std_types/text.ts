// Mutations of src/lower/std_types/text.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "extend-from-within-spread",
    breaks:
      "`v.extend_from_within(..)` of 200,000 items is `v.push(...$copyRange(v, 0))`, a call's spread past JS's bound of arguments: `RangeError` (#334)",
    file: "src/lower/std_types/text.rs",
    find: '            return Expr::call_helper("$extend", vec![items, copied]);\n',
    replace:
      '            return Expr::call(Expr::member(items, "push"), vec![Expr::spread(copied)]);\n',
    tests: ["test/corpus.test.ts", "-t", "^long_spreads.rs$"],
  },
  {
    name: "drain-in-bounds-checked",
    breaks:
      "a `drain(0..i)` the body's ranges show in bounds is `$drain(queue, 0, i)`, not `queue.splice(0, i)`",
    file: "src/lower/std_types/text.rs",
    find: "        if op == TextOp::Drain && in_bounds {\n",
    replace: "        if op == TextOp::Drain && in_bounds && false {\n",
    tests: ["test/corpus.test.ts", "-t", "refcell_vec_bounds"],
    snapshots: true,
  },
  {
    name: "drain-from-zero-subtracted",
    breaks: "`drain(0..i)` in bounds is `queue.splice(0, i - 0)`",
    file: "src/lower/std_types/text.rs",
    find: "                (Some(0), Some(end)) => Some(end),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "refcell_vec_bounds"],
    snapshots: true,
  },
  {
    name: "whole-slice-copied",
    breaks: "`&v[..]` is `$slice(v, 0)`, a copy of all of it",
    file: "src/lower/std_types/text.rs",
    find: '        if name == "$slice" && start.as_int() == Some(0) && end.is_none() {',
    replace:
      '        if false && name == "$slice" && start.as_int() == Some(0) && end.is_none() {',
    tests: ["test/lowering-collections.test.ts", "-t", "shared slice of all"],
  },
  {
    name: "replacen-any-count",
    breaks: "`replacen(p, r, 2)` is JS's `replace`, of the first only",
    file: "src/lower/std_types/text.rs",
    find: "                    if !matches!(rest[2].kind, js::ExprKind::Num(n) if n == 1.0) {",
    replace: "                    if false {",
    tests: ["test/lowering-strings.test.ts", "-t", "replacen replaces as many"],
  },
  {
    name: "replacen-count-given",
    breaks: "`replacen`'s count is given to JS's `replace`, which takes none",
    file: "src/lower/std_types/text.rs",
    find: "                    rest.truncate(2);\n",
    replace: "",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a replacement is the text it is",
    ],
  },
  {
    name: "replacement-read-as-pattern",
    breaks:
      "a replacement's `$&` is what's matched, as JS reads a string's, not the text Rust has",
    file: "src/lower/std_types/text.rs",
    find: '                if matches!(name, "replaceAll" | "replace") {\n                    rest[1] = replacement(rest[1].clone());',
    replace:
      "                if false {\n                    rest[1] = replacement(rest[1].clone());",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a replacement is the text it is",
    ],
  },
  {
    name: "replacement-literal-as-function",
    breaks:
      "a replacement written out is a function's, `() => 'b'`, where a person writes `'b'`",
    file: "src/lower/std_types/text.rs",
    find: "        js::ExprKind::Str(text) => Expr::str(text.replace('$', \"$$\")),\n",
    replace: "",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a replacement is the text it is",
    ],
  },
  {
    name: "as-bytes-utf16",
    breaks:
      "`s.into_bytes()` is each UTF-16 unit, `é` one byte where UTF-8 has two",
    file: "src/lower/std_types/text.rs",
    find: "    let encoded = Expr::call(Expr::member(encoder, \"encode\"), vec![text]);\n    Expr::builtin(Expr::js_global(\"Array\"), \"from\", vec![encoded])\n}\n",
    replace: "    let _ = encoder;\n    Expr::builtin(Expr::js_global(\"Array\"), \"from\", vec![text, Expr::arrow(vec![\"c\".into()], vec![crate::js::StmtKind::Return(Some(Expr::builtin(Expr::var(\"c\"), \"charCodeAt\", vec![Expr::int(0)]))).at(crate::js::Span::NONE)])])\n}\n",
    tests: ["test/corpus.test.ts", "-t", "^text_bytes_lent_and_owned.rs$"],
  },
  {
    name: "slice-to-inclusive-end-dropped",
    breaks: "`&v[..=1]` stops before index 1",
    file: "src/lower/std_types/text.rs",
    find: "            (RangeKind::ToInclusive, [end]) => (Expr::int(0), Some(past(end.clone()))),",
    replace:
      "            (RangeKind::ToInclusive, [end]) => (Expr::int(0), Some(end.clone())),",
    tests: ["test/corpus.test.ts", "-t", "range_values"],
  },
  {
    name: "str-full-slice-checked",
    breaks:
      "`&s[..]` of a string is checked by `$strSlice`: right, but not the JS a person writes",
    file: "src/lower/std_types/text.rs",
    find: "        if matches!(op, TextOp::StrSlice | TextOp::StrGet) && start.as_int() == Some(0) && end.is_none() {",
    replace: "        if false {",
    tests: ["test/corpus.test.ts", "-t", "string_bytes"],
    snapshots: true,
  },
  {
    name: "ascii-whitespace-js",
    breaks:
      "`split_ascii_whitespace` splits at JS's `\\s`, a vertical tab and Unicode spaces too",
    file: "src/lower/std_types/text.rs",
    find: 'Expr::regex("/[\\\\t\\\\n\\\\f\\\\r ]+/")',
    replace: 'Expr::regex("/\\\\s+/")',
    tests: ["test/corpus.test.ts", "-t", "text_predicates"],
  },
  {
    name: "char-set-not-predicate",
    breaks:
      "a set of `char`s as a pattern is given as the array, which isn't called",
    file: "src/lower/std_types/text.rs",
    find: "            && item.is_char()\n",
    replace: "            && !item.is_char()\n",
    tests: ["test/corpus.test.ts", "-t", "text_predicates"],
  },
  {
    name: "parse-skips-user-from-str",
    breaks: "`s.parse::<Role>()` of a type's own `FromStr` is an error",
    file: "src/lower/std_types/text.rs",
    find: "        if let Some(from_str) = self.recognition().std_from_str()\n",
    replace:
      "        if let Some(from_str) = self.recognition().std_from_str().filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "user_from_str"],
  },
  {
    name: "parse-generic-target-refused",
    breaks:
      "`s.parse::<T>()` of a generic `T: FromStr` is an error, its dictionary unused",
    file: "src/lower/std_types/text.rs",
    find: "            && (self.is_unknown(target) || self.has_user_impl(from_str, target))",
    replace: "            && self.has_user_impl(from_str, target)",
    tests: ["test/corpus.test.ts", "-t", "generic_from_str"],
  },
  {
    name: "split-last-keeps-last",
    breaks: "`split_last()`'s rest keeps the last item too",
    file: "src/lower/std_types/text.rs",
    find: "vec![Expr::int(0), Expr::int(-1)],",
    replace: "vec![Expr::int(0)],",
    tests: ["test/corpus.test.ts", "-t", "^slice_split.rs"],
  },
  {
    name: "split-whitespace-js-s",
    breaks:
      "`split_whitespace()` splits by JS's `\\s`, at U+FEFF too, where Rust's whitespace is Unicode's `White_Space`",
    file: "src/lower/std_types/text.rs",
    find: '                let words = method(arg(), "split", vec![Expr::regex("/\\\\p{White_Space}+/u")]);',
    replace:
      '                let words = method(arg(), "split", vec![Expr::regex("/\\\\s+/")]);',
    tests: [
      "test/compiler.test.ts",
      "-t",
      "chars, parse and slices are plain JS with Rust's answers",
    ],
  },
  {
    name: "extend-from-within-unspread",
    breaks: "`extend_from_within` pushes the range as one item, not its items",
    file: "src/lower/std_types/text.rs",
    find: '            return Expr::call_helper("$extend", vec![items, copied]);\n',
    replace:
      '            return Expr::call(Expr::member(items, "push"), vec![copied]);\n',
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
  {
    name: "trim-ascii-js-whitespace",
    breaks: "`trim_ascii` trims JS's whitespace, a vertical tab too",
    file: "src/lower/std_types/text.rs",
    find: '                    (true, true) => "/^[\\\\t\\\\n\\\\f\\\\r ]+|[\\\\t\\\\n\\\\f\\\\r ]+$/g",\n',
    replace: '                    (true, true) => "/^\\\\s+|\\\\s+$/g",\n',
    tests: ["test/corpus.test.ts", "-t", "text_edits"],
  },
  {
    name: "str-get-mut-panics",
    breaks: "`get_mut(range)` panics where it's `None`",
    file: "src/lower/std_types/text.rs",
    find: '            TextOp::StrGetMut => "$strGetMut",\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "str-split-at-mut-unchecked",
    breaks: "`split_at_mut_checked` panics where it's `None`",
    file: "src/lower/std_types/text.rs",
    find: '                if checked {\n                    list.push(Expr::bool(true));\n                }\n                "$strSplitAtMut"',
    replace: '                "$strSplitAtMut"',
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "str-slice-bounds-end",
    breaks: "`slice_unchecked(a, b)` runs to the end",
    file: "src/lower/std_types/text.rs",
    find: 'e", vec![arg(), arg(), arg()',
    replace: 'e", vec![arg(), arg()',
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "from-utf8-mut-checked",
    breaks: "`from_utf8_unchecked_mut` gives a `Result`",
    file: "src/lower/std_types/text.rs",
    find: 'ut", list',
    replace: 'ut", vec![list.remove(0)]',
    tests: ["test/corpus.test.ts", "-t", "str_mut"],
  },
  {
    name: "strip-circumfix-suffix-kept",
    breaks: "`strip_circumfix` strips the prefix only",
    file: "src/lower/std_types/text.rs",
    find: 'Expr::call(Expr::helper("$stripCircumfix"), vec![arg(), arg(), arg()])',
    replace: 'Expr::call(Expr::helper("$stripPrefix"), vec![arg(), arg()])',
    tests: ["test/corpus.test.ts", "-t", "str_circumfix"],
  },
  {
    name: "extend-within-index-checked",
    breaks:
      "`extend_from_within(a..b)` is checked as `&v[a..b]` is, its start first",
    file: "src/lower/std_types/text.rs",
    find: '            let copied = Expr::call(Expr::helper("$copyRange"), list);\n',
    replace:
      '            let copied = Expr::call(Expr::helper("$slice"), list);\n',
    tests: ["test/corpus.test.ts", "-t", "extend_within_past_len"],
  },
  {
    name: "starts-with-eq-dropped",
    breaks: "a slice's `starts_with` of structs compares them by identity",
    file: "src/lower/std_types/text.rs",
    find: '                list.extend(eq);\n                call("$sliceStartsWith"',
    replace: '                call("$sliceStartsWith"',
    tests: ["test/corpus.test.ts", "-t", "slice_prefix_eq"],
  },
  {
    name: "ends-with-end-dropped",
    breaks: "`ends_with` of structs is `starts_with`",
    file: "src/lower/std_types/text.rs",
    find: "                if end || eq.is_some() {\n                    list.push(Expr::bool(end));",
    replace:
      "                if end || eq.is_some() {\n                    list.push(Expr::bool(false));",
    tests: ["test/corpus.test.ts", "-t", "slice_prefix_eq"],
  },
  {
    name: "range-value-from-start",
    breaks:
      "a slice by a range kept as a value starts at 0, not at the range's start",
    file: "src/lower/std_types/text.rs",
    find: "            (RangeKind::From, [start]) => (start.clone(), None),\n",
    replace: "            (RangeKind::From, [_]) => (Expr::int(0), None),\n",
    tests: ["test/corpus.test.ts", "-t", "slice_|string_slice"],
  },
  {
    name: "replacen-first-only",
    breaks: "`replacen(p, r, n)` replaces only the first, whatever `n`",
    file: "src/lower/std_types/text.rs",
    find: 'return Ok(Some(Expr::call(Expr::helper("$replacen"), [vec![this], rest].concat())));',
    replace:
      'return Ok(Some(Expr::call(Expr::member(this, "replace"), rest[..2].to_vec())));',
    tests: ["test/lowering-strings.test.ts", "-t", "replacen replaces as many"],
  },
  {
    name: "str-method-unmarked-read",
    breaks:
      "a `str`'s `starts_with` isn't marked as only reading it, and its chain stays an iterator's",
    file: "src/lower/std_types/text.rs",
    find: "                Expr::read(this, name, rest)",
    replace: "                Expr::call(Expr::member(this, name), rest)",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "named as a slice's reader",
    ],
  },
  {
    name: "range-edit-of-all-crashes",
    breaks:
      "`v.extend_from_within(..)`, all of it, which reads the `Vec` itself, crashes the compiler",
    file: "src/lower/std_types/text.rs",
    find: "            js::ExprKind::Call(_, list) => list,\n            kind => vec![\n",
    replace:
      "            js::ExprKind::Call(_, list) => list,\n            _ => unreachable!(\"a range's read is a helper's call\"),\n            #[allow(unreachable_patterns)]\n            kind => vec![\n",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a vec's range pushed again or replaced",
    ],
  },
  {
    name: "splitn-set-pattern-unmade",
    breaks:
      "`splitn(2, ['e', 'E'])`'s set isn't made a predicate, and JS calls the array",
    file: "src/lower/std_types/text.rs",
    find: "        let at = if matches!(op, TextOp::SplitNBy | TextOp::RsplitBy(true)) {\n            2\n        } else {\n            1\n        };\n",
    replace: "        let at = 1;\n",
    tests: ["test/corpus.test.ts", "-t", "^splitn_by.rs$"],
  },
  {
    name: "found-tested-against-null",
    breaks:
      "a place found in units is tested `at != null`, so `indexOf`'s `-1` is found",
    file: "src/lower/std_types/text.rs",
    find: "    Expr::bin(Op::Ne, at, Expr::int(-1))",
    replace: "    Expr::bin(Op::LooseNe, at, Expr::null())",
    tests: ["test/lowering-string-places.test.ts", "-t", "owner stack"],
  },
  {
    name: "cut-suffix-from-the-front",
    breaks:
      "`strip_suffix(p)`'s cut is `s.slice(n)`, what's after the suffix's length, not before it",
    file: "src/lower/std_types/text.rs",
    find: "        (Some(n), false) => slice(vec![Expr::int(0), Expr::int(-(n as i128))]),",
    replace: "        (Some(n), false) => slice(vec![Expr::int(n as i128)]),",
    tests: ["test/lowering-string-places.test.ts", "-t", "strip tested"],
  },
  {
    name: "appends-kept-apart",
    breaks:
      '`stack += "\\n    at "; stack += callSite.toString();` stays two, not one `+=`',
    file: "src/lower/std_types/text.rs",
    find: "        if let Some((s, a)) = append_of(&stmts[i], text)\n",
    replace:
      "        if false\n            && let Some((s, a)) = append_of(&stmts[i], text)\n",
    tests: ["test/lowering-strings.test.ts", "-t", "consecutive appends"],
  },
  {
    name: "appends-reading-it-joined",
    breaks:
      "an append that reads the string, `s += s`, is joined to the one before, and reads it without that one",
    file: "src/lower/std_types/text.rs",
    find: "            && !b.mentions_var(s)\n",
    replace: "",
    tests: ["test/lowering-strings.test.ts", "-t", "consecutive appends"],
  },
  {
    name: "appends-of-two-strings-joined",
    breaks:
      "an append to one string, then one to another, are one, to the first",
    file: "src/lower/std_types/text.rs",
    find: "            && s == t\n",
    replace: "",
    tests: ["test/lowering-strings.test.ts", "-t", "consecutive appends"],
  },
  {
    name: "appends-of-two-values-joined",
    breaks:
      'appends of two values and their text, a line each, are one statement of both, `out += a + "\\n" + b + "\\n"`',
    file: "src/lower/std_types/text.rs",
    find: "            && values(a) + values(b) <= 1\n",
    replace: "",
    tests: ["test/lowering-strings.test.ts", "-t", "consecutive appends"],
  },
  {
    name: "appends-of-text-swapped",
    breaks:
      'two pieces of text written out are joined the other way round, `"?!"`',
    file: "src/lower/std_types/text.rs",
    find: '                (ExprKind::Str(a), ExprKind::Str(b)) => Expr::str(format!("{a}{b}")),\n',
    replace:
      '                (ExprKind::Str(a), ExprKind::Str(b)) => Expr::str(format!("{b}{a}")),\n',
    tests: ["test/lowering-strings.test.ts", "-t", "consecutive appends"],
  },
  {
    name: "matches-by-set-unmade",
    breaks:
      "`matches` of a set of `char`s is given the set, not the predicate of being one, and throws",
    file: "src/lower/std_types/text.rs",
    find: "                | TextOp::MatchesBy(_)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^matches_by_predicate.rs$"],
  },
  {
    name: "char-set-pattern-unconverted",
    breaks:
      "a set of `char`s as `rsplit_once`'s pattern is called as a function",
    file: "src/lower/std_types/text.rs",
    find: "                | TextOp::SplitOnceBy(_)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^char_patterns.rs$"],
  },
  {
    name: "slice-in-bounds-checked",
    breaks:
      "a slice's range the ranges show in bounds is still `$slice(items, 0, 3)`, not JS's own `items.slice(0, 3)` (ADR 0806)",
    file: "src/lower/std_types/text.rs",
    find: '        if name == "$slice" && in_bounds {\n',
    replace: '        if name == "$slice" && in_bounds && false {\n',
    tests: ["test/corpus.test.ts", "-t", "slice_ranges_known"],
    snapshots: true,
  },
  {
    name: "slice-always-unchecked",
    breaks:
      "every slice's range is JS's own `slice`, which clamps, so `&items[5..]` of three items is empty, where Rust panics (ADR 0806)",
    file: "src/lower/std_types/text.rs",
    find: '        if name == "$slice" && in_bounds {\n',
    replace: '        if name == "$slice" {\n',
    tests: ["test/corpus.test.ts", "-t", "slice_range_from_panic"],
  },
  {
    name: "concat-text-unjoined",
    breaks:
      '`s + &js::concat_text(n)` of a `String` is `s + ("" + n)`, its `String` not known text (ADR 0639, amended)',
    file: "src/lower/std_types/text.rs",
    find: "            Std::Concat => Expr::text_joined(arg(), arg()),\n",
    replace: "            Std::Concat => Expr::bin(Op::Add, arg(), arg()),\n",
    tests: ["test/bindings.test.ts", "-t", "any value is read as JS reads it"],
  },
  {
    name: "extended-cows-as-objects",
    breaks: "a `String`'s `extend` of `Cow<str>`s pushes the `Cow`s themselves, `[object Object]` (#495)",
    file: "src/lower/std_types/text.rs",
    find: "        let items = self.cow_texts(items, self.iterable_item(ty));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cow_texts_joined.rs$"],
  },
  {
    name: "joined-text-of-iterable-unarrayed",
    breaks:
      "a string extended by an iterable that isn't an array, generic code's, joins nothing",
    file: "src/lower/std_types/text.rs",
    find: "            false => Expr::builtin(Expr::js_global(\"Array\"), \"from\", vec![items]),\n",
    replace: "            false => items,\n",
    tests: ["test/corpus.test.ts", "-t", "^text_extended.rs$"],
  },
  {
    name: "text-extend-unclosed",
    breaks:
      "a `String`'s `extend` of an iterator whose steps may panic joins its text and appends it once, by `Array.from`, which never closes it where its `next` throws: a panic part-way never drops its source (#1126), and loses what came before it (#1127)",
    file: "src/lower/std_types/text.rs",
    find: "        if !stepped {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_consumed_by_extend_unzip_partition_eq.rs$"],
  },
  {
    name: "joined-text-own-items",
    breaks:
      "a `String` extended by a range of `char`s or an `Option` takes `Array.from` of the range's object or of the `Option`'s value, not their items as `iterable_items` steps them",
    file: "src/lower/std_types/text.rs",
    find: "        let items = self.iterable_items(items, ty, span, out)?;\n        let ty = ty.peel_refs();\n",
    replace: "        let ty = ty.peel_refs();\n",
    tests: ["test/corpus.test.ts", "-t", "^text_extended.rs$"],
  },
  {
    name: "string-extend-unjoined",
    breaks:
      '`s.extend(chars)` adds their array, `s += ["y", "z"]`, not their text',
    file: "src/lower/std_types/text.rs",
    find: "            let text = self.joined_text(items, ty, span, out)?;\n",
    replace: "            let text = items;\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_extend.rs$"],
  },
  {
    name: "as-bytes-encoded-each-time",
    breaks: "`s.as_bytes()` encodes the text each time it's asked, so a parser's loop asking it each step is quadratic, semver's 5.6 MB identifier past twenty minutes (#1184)",
    file: "src/lower/std_types/text.rs",
    find: "            TextOp::Bytes => call(\"$utf8Bytes\", vec![arg()]),\n",
    replace: "            TextOp::Bytes => utf8_bytes(arg()),\n",
    tests: ["test/corpus.test.ts", "-t", "^long_text_read_byte_by_byte.rs$"],
  },
  {
    name: "into-bytes-capacity-known",
    breaks: "`into_bytes()` of a string, in a crate that reads a capacity, gives its length as its capacity, where std's is the string's (#1180)",
    file: "src/lower/std_types/text.rs",
    find: "            TextOp::IntoBytes if self.krate.keeps_capacity => call(\"$untracked\", vec![utf8_bytes(arg())]),\n",
    replace: "",
    tests: ["test/vec-capacity.test.ts", "-t", "capacity\\(\\) of a Vec whose growth wasn't kept"],
  },
  {
    name: "chars-always-js-iterator",
    breaks: "a `chars()` asked its hint is JS's own string iterator, whose hint is `(0, None)`, not the runtime's `$chars(s)`",
    file: "src/lower/std_types/text.rs",
    find: "                match self.chars_asked_whole(made.output()) {\n",
    replace: "                match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^chars_size_hint.rs$"],
  },
  {
    name: "extend-text-callback-kept",
    breaks:
      "`$extendText`'s callback isn't said to run only in its call, so a `String` shadowed after its `extend` is renamed, `let s$1 = \"\"`, where its binding is reused (#1168)",
    file: "src/lower/std_types/text.rs",
    find: "    push.facts = push.facts.with(js::Known::RunsInItsCall);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^extend_reserves_by_hint.rs$"],
    snapshots: true,
  },
  {
    name: "retain-text-unwinding-unasked",
    breaks:
      "a `String`'s `retain` whose closure may panic is JS's `filter`, so its panic leaves the text it had (#1171)",
    file: "src/lower/std_types/text.rs",
    find: "            StringEdit::Retain if unwinds => {\n",
    replace: "            StringEdit::Retain if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^edits_that_panic_keep_what_std_keeps.rs$"],
  },
  {
    name: "closure-search-made-eager",
    breaks: "a `str`'s search by a pattern that runs code is made the array of what it finds, `$splitBy`, its predicate asked of every `char` first (#1501)",
    file: "src/lower/std_types/text.rs",
    find: "if runs_code { lazy } else { eager }",
    replace: "if runs_code { eager } else { eager }",
    tests: ["test/corpus.test.ts", "-t", "^str_searches_ask_a_closure_as_they_step.rs$"],
  },
  {
    name: "array-map-drops-unasked",
    breaks: "an array's `map` of what has a destructor is JS's own `map`, which drops neither what was made nor what's left where its closure panics (#1586)",
    file: "src/lower/std_types/text.rs",
    find: "                    && (self.has_drops(*item) || self.has_drops(output))\n",
    replace: "                    && false\n",
    tests: ["test/corpus.test.ts","-t","^array_map_panicking_part_way_drops_both_sides.rs$"],
  },
  {
    name: "array-map-helper-where-nothing-panics",
    breaks: "an array's `map` whose closure can't panic, `(value) => [value]`, is `$arrayMap` with its drops, where JS's own `map` reads as the source does (#1586)",
    file: "src/lower/std_types/text.rs",
    find: "                    && can_panic\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_deque_retain_swaps.rs$"],
    snapshots: true,
  },
  {
    name: "parse-to-associated-type-refused",
    breaks: "`text.parse::<S::Item>()` of an associated type only a caller knows is refused, where its dictionary's `from_str` parses it (#1605)",
    file: "src/lower/std_types/text.rs",
    find: "            && (self.is_unknown(target) || self.has_user_impl(from_str, target))\n",
    replace: "            && (matches!(target.kind(), ty::Param(_)) || self.has_user_impl(from_str, target))\n",
    tests: ["test/corpus.test.ts", "-t", "^parse_to_an_associated_type.rs$"],
  },
];
