#![deny(warnings)]

// Rust 1.100 stabilizes naming the never type in all type positions and changes
// Infallible into an alias for !. This fixture intentionally compiles under
// editions 2018, 2021 and 2024 because the fallback transition affects older
// editions too.

pub fn unwrap_infallible<T>(result: Result<T, !>) -> T {
    let Ok(value) = result;
    value
}

pub fn return_infallible() -> Result<(), core::convert::Infallible> {
    Ok(())
}

pub fn diverge() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
