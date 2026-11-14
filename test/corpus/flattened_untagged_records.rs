// An untagged enum each of whose variants holds a record, flattened into
// the event that carries it (#1818), as a UI kit's numeric keyboard takes a
// key and its modifiers together with a composition event, direct or
// wrapped by a framework: `{ key, shiftKey, altKey } & CompositionInput`.
// Each variant is made in place and given made elsewhere, told apart where
// it's flattened by the record's test member, lent whole as the parent
// itself, matched and read through.
#[derive(Default)]
pub struct CompositionFields {
    pub is_composing: Option<bool>,
    pub key_code: Option<f64>,
}

pub struct WrappedCompositionEvent<'a> {
    #[cfg_attr(rust_js, rust_js::test)]
    pub event_type: &'a str,
    pub native_event: &'a CompositionFields,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum CompositionInput<'a> {
    Wrapped(WrappedCompositionEvent<'a>),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Direct(&'a CompositionFields),
}

pub struct NumberKeyEvent<'a> {
    pub key: &'a str,
    pub shift_key: Option<bool>,
    pub alt_key: Option<bool>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub composition: CompositionInput<'a>,
}

/// Whether an input method is composing: Safari's legacy key code 229
/// counts even where `is_composing` is false.
pub fn is_composing(input: &CompositionInput) -> bool {
    let fields = match input {
        CompositionInput::Wrapped(wrapped) => wrapped.native_event,
        CompositionInput::Direct(fields) => fields,
    };
    fields.is_composing == Some(true) || fields.key_code == Some(229.0)
}

pub fn handles(event: &NumberKeyEvent) -> bool {
    !is_composing(&event.composition) && event.key.len() == 1 && event.alt_key != Some(true)
}

fn wrapped_type(event: &NumberKeyEvent) -> String {
    if let CompositionInput::Wrapped(wrapped) = &event.composition {
        format!("{} {:?}", wrapped.event_type, wrapped.native_event.key_code)
    } else {
        "direct".to_owned()
    }
}

fn event<'a>(key: &'a str, composition: CompositionInput<'a>) -> NumberKeyEvent<'a> {
    NumberKeyEvent {
        key,
        shift_key: Some(key == "2"),
        alt_key: None,
        composition,
    }
}

fn main() {
    let typing = CompositionFields {
        is_composing: Some(true),
        key_code: None,
    };
    let idle = CompositionFields::default();
    let safari = CompositionFields {
        is_composing: Some(false),
        key_code: Some(229.0),
    };
    let mut events = vec![
        NumberKeyEvent {
            key: "1",
            shift_key: None,
            alt_key: None,
            composition: CompositionInput::Direct(&idle),
        },
        NumberKeyEvent {
            key: "3",
            shift_key: None,
            alt_key: Some(true),
            composition: CompositionInput::Direct(&safari),
        },
        NumberKeyEvent {
            key: "4",
            shift_key: None,
            alt_key: None,
            composition: CompositionInput::Wrapped(WrappedCompositionEvent {
                event_type: "compositionupdate",
                native_event: &typing,
            }),
        },
    ];
    for (i, key) in ["2", "5", "67"].into_iter().enumerate() {
        let composition = match i {
            0 => CompositionInput::Direct(&typing),
            1 => CompositionInput::Wrapped(WrappedCompositionEvent {
                event_type: "keydown",
                native_event: &safari,
            }),
            _ => CompositionInput::Direct(&idle),
        };
        events.push(event(key, composition));
    }
    for event in &events {
        let kind = match &event.composition {
            CompositionInput::Wrapped(_) => "wrapped",
            CompositionInput::Direct(_) => "direct",
        };
        println!(
            "{} {} {} {:?} {}",
            event.key,
            kind,
            handles(event),
            event.shift_key,
            wrapped_type(event)
        );
    }
    let composing = events.iter().filter(|event| is_composing(&event.composition)).count();
    println!("{composing} of {} composing", events.len());
}
