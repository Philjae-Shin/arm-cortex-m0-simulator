; vector_ops_scalar.asm — Scalar-loop equivalent of vector_ops.asm
; Same operations using scalar loops instead of VADD/VMUL/VDOT.
; Used to compare instruction count and cycle count against SIMD version.
;
; Operations (same as vector_ops.asm):
;   1. C[i] = A[i] + B[i]  loop-based
;   2. D[i] = A[i] * B[i]  loop-based
;   3. R3   = dot(A, B)     loop-based accumulate
;
; Memory layout:
;   0x1000: A[0..7]  (8 words)
;   0x1020: B[0..7]  (8 words)
;   0x1040: C[0..7]  (result of addition)
;   0x1060: D[0..7]  (result of multiplication)

.data 0x1000
    arr_a: .word 10, 20, 30, 40, 50, 60, 70, 80
    arr_b: .word 5, 15, 25, 35, 45, 55, 65, 75
    arr_c: .word 0, 0, 0, 0, 0, 0, 0, 0
    arr_d: .word 0, 0, 0, 0, 0, 0, 0, 0

.text
    ; --- Phase 1: C = A + B (scalar loop) ---
    MOVI  R0, #0x1000       ; A base
    MOVI  R1, #0x1020       ; B base
    MOVI  R2, #0x1040       ; C base
    MOVI  R3, #0            ; i = 0
    MOVI  R4, #8            ; limit

add_loop:
    CMP   R3, R4
    BGE   mul_setup

    LDRPOST R5, R0, #4      ; R5 = A[i]; R0 += 4
    LDRPOST R6, R1, #4      ; R6 = B[i]; R1 += 4
    ADD   R7, R5, R6        ; R7 = A[i] + B[i]
    STRPOST R7, R2, #4      ; C[i] = R7; R2 += 4
    ADDI  R3, R3, #1        ; i++

    B     add_loop

    ; --- Phase 2: D = A * B (scalar loop) ---
mul_setup:
    MOVI  R0, #0x1000       ; A base (reset)
    MOVI  R1, #0x1020       ; B base (reset)
    MOVI  R2, #0x1060       ; D base
    MOVI  R3, #0            ; i = 0

mul_loop:
    CMP   R3, R4
    BGE   dot_setup

    LDRPOST R5, R0, #4      ; R5 = A[i]
    LDRPOST R6, R1, #4      ; R6 = B[i]
    MUL   R7, R5, R6        ; R7 = A[i] * B[i]
    STRPOST R7, R2, #4      ; D[i] = R7
    ADDI  R3, R3, #1        ; i++

    B     mul_loop

    ; --- Phase 3: dot(A, B) = sum(A[i] * B[i]) ---
dot_setup:
    MOVI  R0, #0x1000       ; A base (reset)
    MOVI  R1, #0x1020       ; B base (reset)
    MOVI  R3, #0            ; i = 0
    MOVI  R8, #0            ; accumulator = 0

dot_loop:
    CMP   R3, R4
    BGE   done

    LDRPOST R5, R0, #4      ; R5 = A[i]
    LDRPOST R6, R1, #4      ; R6 = B[i]
    MUL   R7, R5, R6        ; R7 = A[i] * B[i]
    ADD   R8, R8, R7        ; accumulator += R7
    ADDI  R3, R3, #1        ; i++

    B     dot_loop

done:
    MOV   R3, R8            ; put result in R3 (same as vector_ops.asm)
    HALT
