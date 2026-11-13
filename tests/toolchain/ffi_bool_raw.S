/*
 * Raw x86_64 SysV ABI observer for the Rust bool-return regression.
 * The wrapper returns the Rust callee's full EAX value as a C uint32_t;
 * it does not normalize the value through C's _Bool conversion.
 */
        .text
        .globl luminous_ffi_bool_true_raw
        .type luminous_ffi_bool_true_raw,@function
luminous_ffi_bool_true_raw:
        .cfi_startproc
        subq    $8, %rsp
        .cfi_adjust_cfa_offset 8
        call    luminous_ffi_bool_true@PLT
        addq    $8, %rsp
        .cfi_adjust_cfa_offset -8
        ret
        .cfi_endproc
        .size luminous_ffi_bool_true_raw, .-luminous_ffi_bool_true_raw

        .globl luminous_ffi_bool_false_raw
        .type luminous_ffi_bool_false_raw,@function
luminous_ffi_bool_false_raw:
        .cfi_startproc
        subq    $8, %rsp
        .cfi_adjust_cfa_offset 8
        call    luminous_ffi_bool_false@PLT
        addq    $8, %rsp
        .cfi_adjust_cfa_offset -8
        ret
        .cfi_endproc
        .size luminous_ffi_bool_false_raw, .-luminous_ffi_bool_false_raw

        .section .note.GNU-stack,"",@progbits
