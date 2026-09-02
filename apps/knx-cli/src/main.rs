//! Headless entry point. Keeping a real CLI alongside the desktop application
//! is what forces the core to stay free of user-interface dependencies and
//! makes import, roundtrip and regression tests runnable in CI without a
//! display.

fn main() {
    println!("knx {}", env!("CARGO_PKG_VERSION"));
}
