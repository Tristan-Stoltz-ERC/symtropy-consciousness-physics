#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>

extern bool luminous_ffi_bool_true(void);
extern bool luminous_ffi_bool_false(void);
extern uint32_t luminous_ffi_bool_true_raw(void);
extern uint32_t luminous_ffi_bool_false_raw(void);

int main(void) {
    /* Ordinary C-level checks remain useful, but _Bool normalizes its value. */
    volatile bool true_value = luminous_ffi_bool_true();
    volatile bool false_value = luminous_ffi_bool_false();

    if (true_value != true) {
        fprintf(stderr, "Rust bool(true) was not observed as C true\n");
        return 1;
    }
    if (false_value != false) {
        fprintf(stderr, "Rust bool(false) was not observed as C false\n");
        return 2;
    }

    /*
     * The assembly wrappers preserve the full EAX return register as uint32_t.
     * This catches non-zero upper bits that C's _Bool conversion can mask.
     */
    volatile uint32_t raw_true = luminous_ffi_bool_true_raw();
    volatile uint32_t raw_false = luminous_ffi_bool_false_raw();

    if (raw_true != 1u) {
        fprintf(stderr, "Rust bool(true) raw EAX was 0x%08x, expected 0x00000001\n",
                raw_true);
        return 3;
    }
    if (raw_false != 0u) {
        fprintf(stderr, "Rust bool(false) raw EAX was 0x%08x, expected 0x00000000\n",
                raw_false);
        return 4;
    }

    puts("ffi_bool_abi=PASS");
    puts("ffi_bool_raw_eax=PASS");
    return 0;
}
