; vector_add_plain.asm — Vector Addition WITHOUT post-indexed addressing
; Computes C[i] = A[i] + B[i] for i = 0..9
; Uses standard LDR/STR + explicit ADDI pointer increment.
; Compare with vector_add.asm (uses LDRPOST/STRPOST) to measure
; the benefit of post-indexed addressing mode.
;
; Memory layout (identical to vector_add.asm):
;   0x1000: A[0..9]  (10 words)
;   0x1028: B[0..9]  (10 words)
;   0x1050: C[0..9]  (result, 10 words)

.data 0x1000
    arr_a: .word 10, 20, 30, 40, 50, 60, 70, 80, 90, 100
    arr_b: .word 5, 15, 25, 35, 45, 55, 65, 75, 85, 95
    arr_c: .word 0, 0, 0, 0, 0, 0, 0, 0, 0, 0

.text
    MOVI  R0, #0x1000      ; A base
    MOVI  R1, #0x1028      ; B base
    MOVI  R2, #0x1050      ; C base
    MOVI  R3, #0            ; i = 0
    MOVI  R4, #10           ; limit

loop:
    CMP   R3, R4            ; if i >= 10, exit
    BGE   done

    LDR   R5, R0, #0        ; R5 = A[i]
    LDR   R6, R1, #0        ; R6 = B[i]
    ADD   R7, R5, R6        ; R7 = A[i] + B[i]
    STR   R7, R2, #0        ; C[i] = R7

    ADDI  R0, R0, #4        ; A pointer += 4
    ADDI  R1, R1, #4        ; B pointer += 4
    ADDI  R2, R2, #4        ; C pointer += 4
    ADDI  R3, R3, #1        ; i++

    B     loop

done:
    HALT
