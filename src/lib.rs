//! A library for acquiring a backtrace at runtime
//!
//! This library is meant to supplement the `RUST_BACKTRACE=1` support of the
//! standard library by allowing an acquisition of a backtrace at runtime
//! programmatically. The backtraces generated by this library do not need to be
//! parsed, for example, and expose the functionality of multiple backend
//! implementations.
//!
//! # Implementation
//!
//! This library makes use of a number of strategies for actually acquiring a
//! backtrace. For example unix uses libgcc's libunwind bindings by default to
//! acquir a backtrace, but dladdr is used on OSX to acquire symbol names while
//! linux uses gcc's libbacktrace.
//!
//! When using the default feature set of this library the "most reasonable" set
//! of defaults is chosen for the current platform, but the features activated
//! can also be controlled at a finer granularity.
//!
//! # Platform Support
//!
//! Currently this library is verified to work on Linux and OSX, but it may work
//! on other platforms as well. Windows support is planned soon!
//!
//! # API Principles
//!
//! This library attempts to be as flexible as possible to accomodate different
//! backend implementations of acquiring a backtrace. Consequently the currently
//! exported functions are closure-based as opposed to the likely expected
//! iterator-based versions. This is done due to limitations of the underlying
//! APIs used from the system.
//!
//! # Usage
//!
//! First, add this to your Cargo.toml
//!
//! ```toml
//! [dependencies]
//! backtrace = "0.1"
//! ```
//!
//! Next:
//!
//! ```
//! extern crate backtrace;
//!
//! fn main() {
//!     backtrace::trace(&mut |frame| {
//!         let ip = frame.ip();
//!         let symbol_address = frame.symbol_address();
//!
//!         // Resolve this instruction pointer to a symbol name
//!         backtrace::resolve(ip, &mut |symbol| {
//!             if let Some(name) = symbol.name() {
//!                 // ...
//!             }
//!             if let Some(filename) = symbol.filename() {
//!                 // ...
//!             }
//!         });
//!
//!         true // keep going to the next frame
//!     });
//! }
//! ```

#![doc(html_root_url = "http://alexcrichton.com/backtrace-rs")]
#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]

extern crate libc;
#[cfg(feature = "kernel32-sys")] extern crate kernel32;
#[cfg(feature = "winapi")] extern crate winapi;
#[cfg(feature = "dbghelp")] extern crate dbghelp;

#[macro_use]
mod macros;

pub use backtrace::{trace, Frame};
mod backtrace;

pub use symbolize::{Symbol, resolve};
mod symbolize;

pub use demangle::demangle;
mod demangle;

#[allow(dead_code)]
struct Bomb {
    enabled: bool,
}

#[allow(dead_code)]
impl Drop for Bomb {
    fn drop(&mut self) {
        if self.enabled {
            panic!("cannot panic during the backtrace function");
        }
    }
}

mod lock {
    use std::cell::Cell;
    use std::mem;
    use std::sync::{Once, Mutex, MutexGuard, ONCE_INIT};

    pub struct LockGuard(MutexGuard<'static, ()>);

    static mut LOCK: *mut Mutex<()> = 0 as *mut _;
    static INIT: Once = ONCE_INIT;
    thread_local!(static LOCK_HELD: Cell<bool> = Cell::new(false));

    impl Drop for LockGuard {
        fn drop(&mut self) {
            LOCK_HELD.with(|slot| {
                assert!(slot.get());
                slot.set(false);
            });
        }
    }

    pub fn lock() -> Option<LockGuard> {
        if LOCK_HELD.with(|l| l.get()) {
            return None
        }
        LOCK_HELD.with(|s| s.set(true));
        unsafe {
            INIT.call_once(|| {
                LOCK = mem::transmute(Box::new(Mutex::new(())));
            });
            Some(LockGuard((*LOCK).lock().unwrap()))
        }
    }
}
