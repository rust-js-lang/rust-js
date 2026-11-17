// Two thread-locals' `with` closures, each applied where it's called, each
// declaring the program's `current`, and the second's `match` binding
// `current` again in its arms: each closure's own is named apart from the
// other's where it's applied, not spelled yet, and of the program still, so
// the one the program declared first in its scope keeps `current` and an
// arm's binding inside it gives way, `current$1`, as React's
// `pushTransition` reads its transition stack (ADR 1150).
use std::cell::RefCell;
use std::rc::Rc;

type Stack = RefCell<Option<Rc<Vec<u32>>>>;

thread_local! {
    static RESUMED: Stack = const { RefCell::new(None) };
    static TRANSITIONS: Stack = const { RefCell::new(None) };
    static PUSHED: RefCell<Vec<Option<Rc<Vec<u32>>>>> = const { RefCell::new(Vec::new()) };
}

fn push(stack: &Stack, value: Option<Rc<Vec<u32>>>) {
    PUSHED.with_borrow_mut(|pushed| pushed.push(value.clone()));
    *stack.borrow_mut() = value;
}

pub fn push_transition(pool: Option<Rc<Vec<u32>>>, new_transitions: Option<Rc<Vec<u32>>>) {
    RESUMED.with(|resumed| match pool {
        None => {
            let current = resumed.borrow().clone();
            push(resumed, current);
        }
        Some(pool) => push(resumed, Some(pool)),
    });
    TRANSITIONS.with(|transitions| {
        let current = transitions.borrow().clone();
        match (current, new_transitions) {
            (None, new_transitions) => push(transitions, new_transitions),
            (current, None) => push(transitions, current),
            (Some(current), Some(new_transitions)) => {
                push(transitions, Some(Rc::new([&current[..], &new_transitions[..]].concat())))
            }
        }
    });
}

fn main() {
    push_transition(None, None);
    push_transition(Some(Rc::new(vec![9])), Some(Rc::new(vec![1])));
    push_transition(None, None);
    push_transition(None, Some(Rc::new(vec![2, 3])));
    PUSHED.with_borrow(|pushed| println!("{pushed:?}"));
}
