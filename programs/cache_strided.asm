; cache_strided.asm — Strided Memory Access Pattern
; Reads from memory with stride=16 words (64 bytes) between accesses.
; This stride causes cache conflict misses in a direct-mapped cache
; when the working set maps to the same cache lines.
;
; Memory layout:
;   0x1000: data block (256 words = 1024 bytes)
;   We access data[0], data[16], data[32], ... (16 elements, stride=64 bytes)

.data 0x1000
    d00: .word 100, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d01: .word 200, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d02: .word 300, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d03: .word 400, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d04: .word 500, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d05: .word 600, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d06: .word 700, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d07: .word 800, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d08: .word 900, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d09: .word 1000, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d10: .word 1100, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d11: .word 1200, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d12: .word 1300, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d13: .word 1400, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d14: .word 1500, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    d15: .word 1600, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0

.text
    MOVI  R0, #0x1000       ; base address
    MOVI  R1, #0             ; accumulator
    MOVI  R2, #0             ; i = 0
    MOVI  R3, #16            ; elements to access
    MOVI  R5, #64            ; stride in bytes (16 words * 4)

    ; Pass 1: strided read
loop1:
    CMP   R2, R3
    BGE   pass2_setup
    LDR   R4, R0, #0         ; load from current address
    ADD   R1, R1, R4
    ADD   R0, R0, R5          ; advance by stride
    ADDI  R2, R2, #1
    B     loop1

    ; Pass 2: re-read same strided pattern
pass2_setup:
    MOVI  R0, #0x1000
    MOVI  R2, #0

loop2:
    CMP   R2, R3
    BGE   pass3_setup
    LDR   R4, R0, #0
    ADD   R1, R1, R4
    ADD   R0, R0, R5
    ADDI  R2, R2, #1
    B     loop2

    ; Pass 3: third pass
pass3_setup:
    MOVI  R0, #0x1000
    MOVI  R2, #0

loop3:
    CMP   R2, R3
    BGE   done
    LDR   R4, R0, #0
    ADD   R1, R1, R4
    ADD   R0, R0, R5
    ADDI  R2, R2, #1
    B     loop3

done:
    HALT
