// A copy of a module's variable, a `let` named as the thread-local it
// reads, is the variable where each of its reads is before anything that
// may run code; a way that ends, a `let else`'s panic, runs nothing what
// follows sees, however it makes its error: React's Parcel client's
// `if (!callServer) { throw new Error(..); } return callServer(id, args);`
// (rd646, ADR 0277, amended). One set between its reads, by a call or in
// place, and one past a way that runs code and goes on, while the other
// ends, stays a copy. So does one read past a `break` out of a labeled
// block that holds a set, which goes on to what follows the block, and
// past a `catch` an unwind's set reaches.
use std::cell::Cell;
use std::panic::catch_unwind;

thread_local! {
    static CALL_SERVER: Cell<Option<fn(u32) -> u32>> = const { Cell::new(None) };
}

pub fn set_server_callback(callback: fn(u32) -> u32) {
    CALL_SERVER.set(Some(callback));
}

fn call_current_server_callback(id: u32) -> u32 {
    let Some(call_server) = CALL_SERVER.get() else {
        panic!("No server callback has been registered.");
    };
    call_server(id)
}

fn call_or_clear(id: u32, clear: bool) -> u32 {
    let Some(call_server) = CALL_SERVER.get() else {
        return 0;
    };
    if clear {
        CALL_SERVER.set(None);
        return 1;
    }
    call_server(id)
}

fn call_then_clear(id: u32) -> u32 {
    let Some(call_server) = CALL_SERVER.get() else {
        panic!("No server callback has been registered.");
    };
    CALL_SERVER.set(None);
    call_server(id)
}

fn call_past_a_reset(id: u32, reset: bool) -> u32 {
    let Some(call_server) = CALL_SERVER.get() else {
        return 0;
    };
    if reset {
        set_server_callback(triple);
    }
    call_server(id)
}

fn reset_or_keep(id: u32, reset: bool) -> u32 {
    let Some(call_server) = CALL_SERVER.get() else {
        return 0;
    };
    if reset {
        set_server_callback(double);
    } else {
        return call_server(id) + 1;
    }
    call_server(id)
}

fn set_then_break(id: u32, swap: bool, more: bool) -> u32 {
    let Some(call_server) = CALL_SERVER.get() else {
        return 0;
    };
    let mut extra = 0;
    'done: {
        if swap {
            CALL_SERVER.set(Some(triple));
            break 'done;
        }
        if more {
            extra = 10;
        }
    }
    call_server(id) + extra
}

fn set_then_unwind(id: u32, swap: bool) -> u32 {
    let Some(call_server) = CALL_SERVER.get() else {
        return 0;
    };
    match catch_unwind(|| {
        if swap {
            CALL_SERVER.set(Some(triple));
            panic!("swapped");
        }
    }) {
        Ok(()) => {}
        Err(_) => return call_server(id) + 100,
    }
    call_server(id)
}

fn double(n: u32) -> u32 {
    n * 2
}

fn triple(n: u32) -> u32 {
    n * 3
}

fn main() {
    println!("{}", call_or_clear(1, false));
    set_server_callback(double);
    println!("{}", call_current_server_callback(3));
    println!("{} {}", call_or_clear(5, false), call_or_clear(5, true));
    println!("{}", CALL_SERVER.get().is_none());
    set_server_callback(double);
    println!("{} {}", call_past_a_reset(7, false), call_past_a_reset(7, true));
    println!("{}", call_past_a_reset(7, false));
    println!("{} {}", reset_or_keep(2, false), reset_or_keep(2, true));
    println!("{}", call_then_clear(4));
    println!("{}", CALL_SERVER.get().is_none());
    set_server_callback(double);
    println!("{} {}", set_then_break(1, false, true), set_then_break(1, false, false));
    println!("{}", set_then_break(1, true, false));
    set_server_callback(double);
    println!("{} {}", set_then_unwind(1, false), set_then_unwind(1, true));
}
