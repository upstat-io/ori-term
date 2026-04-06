//! Binary entry point for the oriterm terminal emulator.
//!
//! Delegates to [`oriterm::run()`] for the actual startup sequence.

// GUI application — no console window on Windows.
#![windows_subsystem = "windows"]

#[cfg(feature = "profile")]
#[global_allocator]
#[allow(unsafe_code)]
static GLOBAL: oriterm::alloc::CountingAlloc = oriterm::alloc::CountingAlloc;

fn main() {
    oriterm::run();
}
