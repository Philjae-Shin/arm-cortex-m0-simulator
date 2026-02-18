; vector_add_unrolled.asm — Unrolled Vector Addition (2x unrolling)
; Computes C[i] = A[i] + B[i] for i = 0..7 with 2x loop unrolling.
; Tests: ILP benefits, reduced branch overhead, superscalar utilisation
;
; Same memory layout as vector_add.asm but processes 2 elements per iteration.

.data 0x1000
    arr_a: .word 10, 20, 30, 40, 50, 60, 70, 80
    arr_b: .word 5, 15, 25, 35, 45, 55, 65, 75
    arr_c: .word 0, 0, 0, 0, 0, 0, 0, 0

.text
    MOVI  R0, #0x1000       ; A base
    MOVI  R1, #0x1020       ; B base (0x1000 + 8*4)
    MOVI  R2, #0x1040       ; C base (0x1020 + 8*4)
    MOVI  R3, #0            ; i = 0
    MOVI  R4, #8            ; limit

loop_unrolled:
    CMP   R3, R4
    BGE   done

    ; --- Element 0 ---
    LDR   R5, [R0, #0]
    LDR   R6, [R1, #0]
    ADD   R7, R5, R6
    STR   R7, [R2, #0]

    ; --- Element 1 ---
    LDR   R5, [R0, #4]
    LDR   R6, [R1, #4]
    ADD   R7, R5, R6
    STR   R7, [R2, #4]

    ADDI  R0, R0, #8       ; advance pointers by 2 elements
    ADDI  R1, R1, #8
    ADDI  R2, R2, #8
    ADDI  R3, R3, #2       ; i += 2

    B     loop_unrolled

done:
    HALT
