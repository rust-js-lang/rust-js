// Mutations of src/js/name.rs (ADR 0093): a variable's name, one form for
// now, its spelling (#1681 change 3, M3, S1).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "spelled-later-apart-of-the-item",
    breaks:
      "a closure's own `current`, applied where it's called, is no longer the program's, so an arm's `current` inside it takes the word, the outer `current$1` (ADR 1150)",
    file: "src/js/name.rs",
    find: "        Name(crate::names::made(base, k), self.1, Id::fresh(), Id::WORD)\n",
    replace: "        Name(crate::names::made(base, k), Of::Item, Id::fresh(), Id::WORD)\n",
    tests: ["test/corpus.test.ts", "-t", "^applied_body_keeps_the_programs_name.rs$"],
    snapshots: true,
  },
  {
    name: "body-item-not-identified",
    breaks: "a body's item, a `fn` the body declares, is asked no capture: a read of it where a nearer local of its spelling takes it isn't refused, a wrong answer unrefused (#2034)",
    file: "src/js/name.rs",
    find: "        self.has_id() || matches!(self.1, Of::BodyItem(..))\n",
    replace: "        self.has_id()\n",
    unitTests: "js::validate::tests::a_read_of_a_body_item_is_of_that_item",
  },
  {
    name: "made-k-not-free",
    breaks: "two guards' fields of one key in a function, `name`, are made one name, `name#1`, which preparation spells as one variable, declared twice (`Name::free_made_k`, ADR 1150)",
    file: "src/js/name.rs",
    find: "        (1..).find(|k| !taken.contains(k)).expect(\"a free one\")\n",
    replace: "        let _ = taken;\n        1\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once is its body where it's dropped"],
  },
  {
    name: "value-suffix-tail-from-a-dollar",
    breaks: "a key's entry's value, `keyValue`, is told its suffix from a `$` it has none of, `Value` lost, as a flag's is",
    file: "src/js/name.rs",
    find: "            Suffix::Value => base.len().checked_sub(suffix.text().len()).map(|at| &base[at..]),\n",
    replace: "            Suffix::Value => base.find('$').map(|at| &base[at..]),\n",
    unitTests: "js::name::tests::the_value_suffix_is_its_text",
  },
  {
    name: "suffix-root-keeps-a-kept-words-dollar",
    breaks: "a drop flag of a word JS keeps, `new`, is `new$$$0$live`, its suffix taken from the `$` of `new$`",
    file: "src/js/name.rs",
    find: "        base.strip_suffix('$').unwrap_or(base)\n",
    replace: "        base\n",
    tests: ["test/corpus.test.ts", "-t", "^arm_temporaries.rs$"],
    snapshots: true,
  },
  {
    name: "flag-of-a-temporary-kept-of-it",
    breaks:
      "a temporary's drop flag records the temporary it was made of, so tidy never names it for the variable its drop is of, `_14$live`",
    file: "src/js/name.rs",
    find: "            Ok(source) if source.is_temporary() || source.is_unread() => (source.suffix_root(), Id::WORD),\n",
    replace: "            Ok(source) if false => (source.suffix_root(), Id::WORD),\n",
    tests: ["test/corpus.test.ts", "-t", "^(option_combinator_drops|combinators_owned_drops)\\.rs$"],
    snapshots: true,
  },
  {
    name: "holder-of-a-temporary-not-one",
    breaks:
      "a holder lowering makes of a temporary, `_5`, isn't a temporary, so tidy never names it for what it holds, nor folds it as one",
    file: "src/js/name.rs",
    find: "            Of::Temporary | Of::Unread => Of::Temporary,\n",
    replace: "            Of::Temporary | Of::Unread => Of::Item,\n",
    tests: ["test/corpus.test.ts", "-t", "^(untagged_field_of_each_variant|store_get_mut)\\.rs$"],
    snapshots: true,
  },
  {
    name: "local-named-as-a-helper-is-one",
    breaks:
      "a local the program names as a helper is spelled, `rust_js::name = \"$some\"`, is taken for the helper, its calls dropped as doing nothing: `0` where native Rust prints `5` (#2016)",
    file: "src/js/name.rs",
    find: "        (self.is_module() && crate::runtime::exports(&self.0)).then_some(self.0.as_str())\n",
    replace: "        crate::runtime::exports(&self.0).then_some(self.0.as_str())\n",
    tests: ["test/corpus.test.ts", "-t", "^local_named_as_a_helper.rs$"],
  },
  {
    name: "name-unlike-its-word",
    breaks: "a name compared with a word, `Var(v) if v.is(\"Boolean\")`, is never it: what a pass asks of a given name, a global's or a helper's, is never so",
    file: "src/js/name.rs",
    find: "    pub fn is(&self, word: &str) -> bool {\n        self.0 == word\n    }\n",
    replace: "    pub fn is(&self, word: &str) -> bool {\n        self.0 != word\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^(std_map|user_deref|clone_from_generic_std_own).rs$"],
  },
  {
    name: "name-as-no-word",
    breaks: "a name's word, what a pass asks of it, `names.contains(v.word())`, is nothing, so no name is the one it asks of",
    file: "src/js/name.rs",
    find: "    pub fn word(&self) -> &str {\n        &self.0\n    }\n",
    replace: "    pub fn word(&self) -> &str {\n        \"\"\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^(std_map|user_deref|clone_from_generic_std_own).rs$"],
  },
  {
    name: "given-last-of-another-name",
    breaks:
      "`const value = make(); x = value;` is `x = make()`, the program's `value` lost",
    file: "src/js/name.rs",
    find: "            Id::WORD => false,\n            origin => origin == of.kin(),\n",
    replace: "            Id::WORD => true,\n            origin => origin == of.kin(),\n",
    tests: ["test/corpus.test.ts", "-t", "^cache_node_walk.rs$"],
    snapshots: true,
  },
];
