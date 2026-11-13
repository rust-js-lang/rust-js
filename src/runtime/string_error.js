// std's error of a message, which `Box<dyn Error>::from("..")` makes: a
// `dyn Error`'s dictionary, whose `value` is the message (ADR 0141).
// Its `type_id` is std's private `StringError`'s, which no program can
// name, so no downcast finds it: a key no type's name is (ADR 0331). Its
// `Display` is its message's, `pad`, given the `Formatter`'s options (#1689);
// written into a writer, `pad`'s writes (#1652).
function $stringError() {
  return {
    Debug: () => ({ fmt: (message) => $debugStr(message) }),
    Display: () => ({ fmt: (message, options) => $formatted(message, options), fmtInto: $writePadded }),
    source: () => undefined,
    typeId: () => "<std's StringError>",
  };
}
