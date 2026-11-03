// Mutations of src/lower/classes/forms.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "es-class-a-constructor-function",
    breaks: "a class marked `#[rust_js::class(es)]` is a constructor function, `function Plugin(..) { .. }`, where upstream writes `class Plugin` (rd682)",
    file: "src/lower/classes/forms.rs",
    find: "    is_class(tcx, adt) && (class_form(tcx, adt).iter().any(|word| word == ES) || superclass(tcx, adt).is_some())\n",
    replace: "    is_class(tcx, adt) && superclass(tcx, adt).is_some()\n",
    tests: ["test/es-classes.test.ts", "-t", "whose struct says es is an ES class"],
  },
  {
    name: "es-class-of-a-subclass-allowed",
    breaks: "`#[rust_js::class(es)]` of a class that extends a binding's compiles, a second way to say what its impl says (rd682)",
    file: "src/lower/classes/forms.rs",
    find: "        [word, ..] if word == ES && extends => {\n",
    replace: "        [word, ..] if false && extends => {\n",
    tests: ["test/binding-subclasses.test.ts", "-t", "marked as the trait marks it"],
  },
  {
    name: "class-form-of-any-word",
    breaks: "`#[rust_js::class(function)]`, a word that's no form, compiles as the class it would be unmarked (rd682)",
    file: "src/lower/classes/forms.rs",
    find: "        [word] if word == ES => None,\n",
    replace: "        _ => None,\n",
    tests: ["test/es-classes.test.ts", "-t", "what an ES class is refused"],
  },
  {
    name: "class-form-on-its-impl",
    breaks: "`#[rust_js::class(es)]` on a class's impl compiles and says nothing, its struct's attribute the form's (rd682)",
    file: "src/lower/classes/forms.rs",
    find: "    (!class_form(tcx, imp).is_empty()).then_some(\n",
    replace: "    (false).then_some(\n",
    tests: ["test/es-classes.test.ts", "-t", "what an ES class is refused"],
  },
  {
    name: "fields-word-unread",
    breaks: "`#[rust_js::class(es, fields)]` is an ES class that declares none of its fields (#568)",
    file: "src/lower/classes/forms.rs",
    find: "    is_class(tcx, adt) && matches!(class_form(tcx, adt).as_slice(), [es, fields] if es == ES && fields == FIELDS)\n",
    replace: "    is_class(tcx, adt) && matches!(class_form(tcx, adt).as_slice(), [es, fields] if es == ES && fields == ES)\n",
    tests: ["test/es-classes.test.ts", "-t", "whose struct says es is an ES class"],
  },
  {
    name: "fields-of-a-subclass-allowed",
    breaks: "`class(es, fields)` of a class that extends a binding's compiles, its fields declared after `super(..)` over the accessor the class it extends may have of a field's name (#568)",
    file: "src/lower/classes/forms.rs",
    find: "        [word, ..] if word == ES && extends => {\n",
    replace: "        [word] if word == ES && extends => {\n",
    tests: ["test/binding-subclasses.test.ts", "-t", "marked as the trait marks it"],
  },
  {
    name: "fields-without-es-allowed",
    breaks: "`#[rust_js::class(fields)]` isn't said to be an ES class's, a constructor function having no field declarations (#568)",
    file: "src/lower/classes/forms.rs",
    find: "        words if words.iter().any(|word| word == FIELDS) => Some(\n",
    replace: "        words if words.is_empty() => Some(\n",
    tests: ["test/es-classes.test.ts", "-t", "fields that would change what it does are refused"],
  },
];
