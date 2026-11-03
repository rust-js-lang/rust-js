// Mutations of src/lower/untagged/displayed.rs (ADR 0093): an untagged
// enum whose `Display` writes each variant's one value, a plain `{}` of it
// the value in JS's template, `${id}` (#557).
import type { Mutation } from "../../../mutations";

const tests = ["test/corpus.test.ts", "-t", "^untagged_displayed_in_place.rs$"];

export const mutations: Mutation[] = [
  {
    name: "displayed-value-of-a-float",
    breaks:
      "an `f64`'s arm is taken as written as it is, so `{}` of `Measure::Size(1e21)` shows `1e+21` (#557)",
    file: "src/lower/untagged/displayed.rs",
    find: "|| shown.is_integral() ||",
    replace: "|| shown.is_numeric() ||",
    tests,
  },
  {
    name: "displayed-value-with-options",
    breaks:
      'a `write!(f, "{number:>3}")` arm is taken as written as it is, so `Padded::Number(4)` shows `4`, not `  4` (#557)',
    file: "src/lower/untagged/displayed.rs",
    find: "        *spec == Spec::plain()\n",
    replace: "        true\n",
    tests,
  },
  {
    name: "displayed-value-with-text",
    breaks:
      'a `write!(f, "#{number}")` arm is taken as written as it is, so `Labeled::Number(3)` shows `3`, not `#3` (#557)',
    file: "src/lower/untagged/displayed.rs",
    find: ".filter(|piece| !matches!(piece, Piece::Text(text) if text.is_empty()))",
    replace: ".filter(|piece| !matches!(piece, Piece::Text(_)))",
    tests,
  },
  {
    name: "displayed-value-of-debug",
    breaks:
      'a `write!(f, "{name:?}")` arm is taken as written as it is, so `Quoted::Name("q")` shows `q`, not `"q"` (#557)',
    file: "src/lower/untagged/displayed.rs",
    find: "fmt_trait_called(self.tcx, made_by) == Some(self.display_trait())",
    replace: "fmt_trait_called(self.tcx, made_by).is_some()",
    tests,
  },
  {
    name: "displayed-value-uncaptured",
    breaks:
      "a `write!(f, \"{name}\")` arm's argument, `format_args!`'s `&*args.0`, isn't followed to its `name`, so `{id}` of webpack's module id is `moduleIdDisplay_fmt(id)` (#557)",
    file: "src/lower/untagged/displayed.rs",
    find: "is_var(self.tcx, thir, captured(thir, value).unwrap_or(value), payload)",
    replace: "is_var(self.tcx, thir, value, payload)",
    tests,
    snapshots: true,
  },
  {
    name: "displayed-value-not-written-str",
    breaks:
      "an `f.write_str(x)` arm isn't taken, so `{boxed}` is `boxedDisplay_fmt(boxed)` (#557)",
    file: "src/lower/untagged/displayed.rs",
    find: "                        && (is_var(self.tcx, thir, written, payload)\n",
    replace: "                        && (false\n",
    tests,
    snapshots: true,
  },
  {
    name: "displayed-value-not-through-deref",
    breaks:
      "an `f.write_str(name)` of a `String`, through its `Deref`, isn't taken, so `{chunk}` is `chunkDisplay_fmt(chunk)` (#557)",
    file: "src/lower/untagged/displayed.rs",
    find: ".is_some_and(|t| tcx.is_lang_item(t, LangItem::Deref)) =>",
    replace: ".is_some_and(|t| tcx.is_lang_item(t, LangItem::DerefMut)) =>",
    tests,
    snapshots: true,
  },
];
