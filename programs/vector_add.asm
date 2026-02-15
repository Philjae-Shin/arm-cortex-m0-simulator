; vector_add.asm — Vector Addition Benchmark
; Computes C[i] = A[i] + B[i] for i = 0..9
; Tests: loop control, memory loads/stores, arithmetic
;
; Memory layout:
;   0x1000: A[0..9]  (10 words)
;   0x1028: B[0..9]  (10 words)
;   0x1050: C[0..9]  (result, 10 words)

.data 0x1000
    arr_a: .word 10, 20, 30, 40, 50, 60, 70, 80, 90, 100
    arr_b: .word 5, 15, 25, 35, 45, 55, 65, 75, 85, 95
    arr_c: .word 0, 0, 0, 0, 0, 0, 0, 0, 0, 0

.text
    ; R0 = base address of A
    ; R1 = base address of B
    ; R2 = base address of C
    ; R3 = loop counter (i)
    ; R4 = loop limit (10)
    ; R5 = A[i]
    ; R6 = B[i]
    ; R7 = C[i] = A[i] + B[i]

    MOVI  R0, #0x1000      ; A base
    MOVI  R1, #0x1028      ; B base  (0x1000 + 10*4)
    MOVI  R2, #0x1050      ; C base  (0x1028 + 10*4)
    MOVI  R3, #0            ; i = 0
    MOVI  R4, #10           ; limit

loop:
    CMP   R3, R4            ; if i >= 10, exit
    BGE   done

    LDR   R5, [R0, #0]     ; R5 = A[i]
    LDR   R6, [R1, #0]     ; R6 = B[i]
    ADD   R7, R5, R6        ; R7 = A[i] + B[i]
    STR   R7, [R2, #0]     ; C[i] = R7

    ADDI  R0, R0, #4       ; advance A pointer
    ADDI  R1, R1, #4       ; advance B pointer
    ADDI  R2, R2, #4       ; advance C pointer
    ADDI  R3, R3, #1       ; i++

    B     loop

done:
    HALT
