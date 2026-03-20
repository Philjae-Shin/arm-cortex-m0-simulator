; cache_sequential.asm — Sequential Memory Access Pattern
; Reads 32 consecutive words from memory in order (stride=1 word).
; Expected: high cache hit rate after initial compulsory misses.
;
; Memory layout:
;   0x1000: data[0..31]  (32 words)

.data 0x1000
    data: .word 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32

.text
    MOVI  R0, #0x1000       ; base address
    MOVI  R1, #0             ; accumulator
    MOVI  R2, #0             ; i = 0
    MOVI  R3, #32            ; limit

    ; Pass 1: sequential read (compulsory misses)
loop1:
    CMP   R2, R3
    BGE   pass2_setup
    LDRPOST R4, R0, #4       ; load data[i], advance pointer
    ADD   R1, R1, R4         ; accumulate
    ADDI  R2, R2, #1
    B     loop1

    ; Pass 2: re-read same data (should all hit)
pass2_setup:
    MOVI  R0, #0x1000        ; reset base
    MOVI  R2, #0             ; i = 0

loop2:
    CMP   R2, R3
    BGE   pass3_setup
    LDRPOST R4, R0, #4
    ADD   R1, R1, R4
    ADDI  R2, R2, #1
    B     loop2

    ; Pass 3: third pass (temporal locality)
pass3_setup:
    MOVI  R0, #0x1000
    MOVI  R2, #0

loop3:
    CMP   R2, R3
    BGE   done
    LDRPOST R4, R0, #4
    ADD   R1, R1, R4
    ADDI  R2, R2, #1
    B     loop3

done:
    HALT
