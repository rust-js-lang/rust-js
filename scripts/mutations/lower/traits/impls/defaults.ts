// Mutations of src/lower/traits/impls/defaults.rs (ADR 0093), split from
// impls.rs's with it (#1334).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "default-own-unparameterized",
    breaks:
      "a copied generic default, `describe<T: Display>`, takes no `TDisplay`, which its body reads",
    file: "src/lower/traits/impls/defaults.rs",
    find: "                Slot::Bound(bound) => own_params.iter().find(|&&(b, _)| b == bound).map(|(_, n)| n.clone()),\n",
    replace: "                Slot::Bound(_) => continue,\n",
    tests: ["test/corpus.test.ts", "-t", "generic_trait_methods"],
  },
  {
    name: "default-without-impl-evidence",
    breaks:
      "a default copied into a generic impl gives the impl's methods a clone that copies nothing, not the impl's `SClone`",
    file: "src/lower/traits/impls/defaults.rs",
    find: "        specialized.extend(self.given.evidence.iter().cloned());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "evidence_paths"],
  },
  {
    name: "default-own-bound-trait-terms",
    breaks:
      "a default's `where Self: PartialEq` isn't found as the impl's type's",
    file: "src/lower/traits/impls/defaults.rs",
    find: "                if concrete != bound {\n                    specialized.push((concrete, Expr::var(&name)));",
    replace:
      "                if false {\n                    specialized.push((concrete, Expr::var(&name)));",
    tests: ["test/corpus.test.ts", "-t", "trait_default_bounds"],
  },
  {
    name: "copied-default-location-param-lost",
    breaks:
      "a copied default that reads its caller's location takes no `location`, and reads one it wasn't given",
    file: "src/lower/traits/impls/defaults.rs",
    find: "                Slot::Location => location.clone(),\n",
    replace: "                Slot::Location => continue,\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller_traits.rs$"],
  },
  {
    name: "default-own-facts-untaken",
    breaks: "a copied default takes none of the facts its caller gives",
    file: "src/lower/traits/impls/defaults.rs",
    find: "                Slot::Fact(index, fact) => (fact_names.iter())\n",
    replace: "                Slot::Fact(..) => continue,\n                Slot::Fact(index, fact) => (fact_names.iter())\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_maps.rs$"],
  },
  {
    name: "class-default-fields-unread",
    breaks:
      "a trait default on a class's prototype calls the method that gives a field, `this.counter().value`, not the field (rd283)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "                mir.read_fields(self.tcx, ty::TypingEnv::fully_monomorphized());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_methods.rs$"],
    snapshots: true,
  },
  {
    name: "default-own-drops-not-taken",
    breaks:
      "a copied default takes none of its own type parameters' drops its caller gives, and never drops what it's given of them (#802)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "        for index in self.own_drops_taken(id, id) {\n",
    replace: "        for index in Vec::<u32>::new() {\n",
    tests: ["test/corpus.test.ts", "-t", "^default_own_param_drops.rs$"],
  },
  {
    name: "default-own-drops-unused-kept",
    breaks:
      "a copied default takes each of its own drops after the last it uses, which nothing reads (#802)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "        params.truncate(params.len() - unused);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^default_method_clones.rs$"],
    snapshots: true,
  },
  {
    name: "default-own-facts-named-by-index",
    breaks:
      "a copied default's own facts are named by their parameter alone, so its `U`'s `Size` and `Align` are one name twice, which JS refuses (#804)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "                    .find(|&&(at, _)| at == (index, fact))\n",
    replace: "                    .find(|&&(at, _)| at.0 == index)\n",
    tests: ["test/corpus.test.ts", "-t", "^default_own_param_numbered_as_the_impls.rs$"],
  },
  {
    name: "default-body-in-trait-terms",
    breaks:
      "a copied default's body is lowered in its trait's terms, its `Self` a type parameter's JS iterator where the impl's is the crate's `Wrapper<I>`, stepped by its own `next`: `self.map(Some).collect()` of a `Wrapper(Countdown(20))` gives 1 item, not 2 (#803)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "            let mut mir = default_mir(self.tcx, mir, args, default_typing_env(self.tcx, imp, id, args));\n",
    replace: "            let mut mir = mir.instantiated(self.tcx, ty::GenericArgs::identity_for_item(self.tcx, id));\n",
    tests: ["test/corpus.test.ts", "-t", "^(default_of_a_non_blanket_impl_steps_its_self).rs$"],
  },
  {
    name: "default-without-impls-drops",
    breaks:
      "a copied default, in its impl's terms, drops with none of its impl's drops, so what it holds of the impl's parameters is never dropped (#803)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "        let (mut drops, mut slots) = self.impl_drops_for_default();\n",
    replace: "        let (mut drops, mut slots) = (IndexMap::<Ty<'tcx>, crate::lower::drops::GivenDrop>::default(), IndexSet::default());\n",
    tests: ["test/corpus.test.ts", "-t", "^(default_of_a_non_blanket_impl_steps_its_self|drop_impl_indirect).rs$"],
  },
  {
    name: "default-box-not-taken-out",
    breaks:
      "a copied default, whose body takes the impl's `&mut C` as the object, is given generic code's box of it, and bumps the box (#803)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "            for _ in 0..forms.unboxed[i] {\n                value = Expr::held(value);\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(generic_mut_ref|dyn_mut).rs$"],
  },
  {
    name: "default-param-const-unless-reassigned",
    breaks:
      "a copied default's parameter taken out of its box is a `let` though its body never assigns it again (#803)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "                    false => StmtKind::Const(name.clone(), value),\n",
    replace: "                    false => StmtKind::Let(name.clone(), Some(value)),\n",
    tests: ["test/corpus.test.ts", "-t", "^(dyn_mut).rs$"],
    snapshots: true,
  },
  {
    name: "default-without-impl-consts",
    breaks:
      "a copied default, in its impl's terms, isn't given the impl's const parameters its `Self` names, arrayvec's `CAP`, and is refused",
    file: "src/lower/traits/impls/defaults.rs",
    find: "        let mut consts = self.impl_const_params();\n",
    replace: "        let mut consts: Vec<(u32, Expr)> = Vec::new();\n",
    tests: ["test/corpus.test.ts", "-t", "^(trait_default_impl_const|default_const_numbered_as_the_impls).rs$"],
  },
  {
    name: "default-own-const-in-trait-numbers",
    breaks:
      "a copied default's own const parameter is given under its trait's number, not the impl's terms' (#804)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "                consts.push((there.index, Expr::var(&name)));\n",
    replace: "                consts.push((index, Expr::var(&name)));\n",
    tests: ["test/corpus.test.ts", "-t", "^(array_defaults|const_generic_traits|default_const_numbered_as_the_impls).rs$"],
  },
  {
    name: "default-own-drops-in-trait-terms",
    breaks:
      "a copied default's own drop is keyed by its slot's type in its trait's terms, not the impl's its body is in, so its body never finds it and drops nothing of its own parameters (#802, #887)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "            drops.insert(in_impl, crate::lower::drops::GivenDrop::lent(name.clone()));\n",
    replace: "            drops.insert(ty, crate::lower::drops::GivenDrop::lent(name.clone()));\n",
    tests: ["test/corpus.test.ts", "-t", "^default_"],
  },
  {
    name: "default-entry-names-after-its-body",
    breaks:
      "a copied default's parameter its entry takes out of a box is named after its body's names are, so it's named as one of them, `self` twice, which JS refuses (#803)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "            let given_names: Vec<Option<String>> = (words.iter())\n                .map(|word| word.map(|word| cx.fresh(Base::Named(word))))\n                .collect();\n",
    replace: "            let given_names: Vec<Option<String>> = (words.iter()).map(|word| word.map(String::from)).collect();\n",
    tests: ["test/corpus.test.ts", "-t", "^(blanket_iterator_trait|generic_dictionary_declared_once|copied_default_box_named_apart).rs$"],
  },
  {
    name: "library-class-default-unasked",
    breaks:
      "a library's trait of classes' default, its consumer's class's prototype's method, isn't compiled of its library's MIR, so the consumer is refused `a trait's default of another crate` (#1073)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "            None if self.asked_of_library(id) => {\n",
    replace: "            None if false => {\n",
    tests: ["test/library-classes.test.ts", "-t", "on the prototype"],
  },
  {
    name: "copied-default-rebound-self-ignored",
    breaks: "a trait's default copied into an impl for `&mut Bag`, `*self = Self::fresh()`, is given generic code's box of the cell as it is, not as its caller holds it (#1732)",
    file: "src/lower/traits/impls/defaults.rs",
    find: "            if let Some(crossing) = forms.rebound[i] {\n",
    replace: "            if let Some(crossing) = forms.rebound[i].filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_impl_replaced_through_generic_code.rs$"],
  },
];
