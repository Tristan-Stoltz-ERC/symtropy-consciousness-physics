// Minimal cross-language ABI probe for the Rust 1.99.1 x86_64 FFI bool fix.
// Compiled as a cdylib and called by the C harness in ffi_bool.c.

#[no_mangle]
pub extern "C" fn luminous_ffi_bool_true() -> bool {
    true
}

#[no_mangle]
pub extern "C" fn luminous_ffi_bool_false() -> bool {
    false
}
