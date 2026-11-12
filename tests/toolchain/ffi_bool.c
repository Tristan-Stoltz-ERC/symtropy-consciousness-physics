#include <stdbool.h>
#include <stdio.h>

extern bool luminous_ffi_bool_true(void);
extern bool luminous_ffi_bool_false(void);

int main(void) {
    /* Preserve the byte returned across the C ABI before checking it. */
    volatile unsigned char true_value =
        (unsigned char)luminous_ffi_bool_true();
    volatile unsigned char false_value =
        (unsigned char)luminous_ffi_bool_false();

    if (true_value != 1) {
        fprintf(stderr, "Rust bool(true) returned byte %u, expected 1\n",
                (unsigned)true_value);
        return 1;
    }
    if (false_value != 0) {
        fprintf(stderr, "Rust bool(false) returned byte %u, expected 0\n",
                (unsigned)false_value);
        return 2;
    }

    puts("ffi_bool_abi=PASS");
    return 0;
}
