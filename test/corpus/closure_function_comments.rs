// A function declared in a closure's body, react-markup's
// `handleFlightError` and `handleError` in `new Promise((resolve, reject)
// => ..)`: each comment is where its code is, before its statement, inside
// a branch, or after a blank line; not inside the first statement's call,
// after it, or above the function (rd557).
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

fn render(f: impl FnOnce(&dyn Fn(Option<&'static str>))) {
    f(&|error| println!("rejected {error:?}"))
}

fn report(on_error: &dyn Fn(Option<&'static str>) -> Option<String>, error: Option<&'static str>) {
    println!("digest {:?}", on_error(error));
}

fn main() {
    render(|reject| {
        let stash_error_idx = Cell::new(1.0);
        let stashed_errors: Rc<RefCell<HashMap<String, Option<&'static str>>>> = Rc::new(RefCell::new(HashMap::new()));

        let handle_flight_error = {
            let stashed_errors = Rc::clone(&stashed_errors);
            move |error: Option<&'static str>| -> Option<String> {
                // We stash the error with a digest in case we need to get
                // to the original error.
                let id = stash_error_idx.get().to_string();
                stash_error_idx.set(stash_error_idx.get() + 1.0);
                stashed_errors.borrow_mut().insert(id.clone(), error);
                Some(id)
            }
        };

        let handle_error = |digest: &'static str| -> Option<String> {
            let mut error = Some(digest);
            // Note that the original error might be `None`.
            if stashed_errors.borrow().contains_key(digest) {
                // Get the original error.
                error = stashed_errors.borrow()[digest];
            }

            // Any error rejects.
            reject(error);
            None
        };

        report(&handle_flight_error, Some("boom"));
        report(&handle_flight_error, None);
        for digest in ["1", "2", "3"] {
            println!("handled {:?}", handle_error(digest));
        }
    });
}
