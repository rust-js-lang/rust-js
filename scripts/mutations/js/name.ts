// Mutations of src/js/name.rs (ADR 0093): a variable's name, one form for
// now, its spelling (#1681 change 3, M3, S1).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
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
