; cache_conflict.asm — Cache Conflict Miss Pattern
; Alternately accesses two addresses that map to the SAME cache line,
; causing 100% conflict misses after initial compulsory misses.
;
; With a 256-line direct-mapped cache (word-indexed), addresses that
; differ by exactly 256 words (1024 bytes) map to the same cache line.
; We access addr 0x1000 and addr 0x1400 alternately (delta = 0x400 = 1024).
;
; Memory layout:
;   0x1000: block_a (first word = 42)
;   0x1400: block_b (first word = 99)

.data 0x1000
    block_a: .word 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0

.data 0x1400
    block_b: .word 99, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0

.text
    MOVI  R0, #0x1000       ; address A
    MOVI  R1, #0x1400       ; address B (same cache index, different tag)
    MOVI  R2, #0             ; accumulator
    MOVI  R3, #0             ; i = 0
    MOVI  R4, #32            ; iterations (32 pairs = 64 accesses)

    ; Alternately load from A and B — every access is a conflict miss
conflict_loop:
    CMP   R3, R4
    BGE   done

    LDR   R5, R0, #0         ; load from A → evicts B's line
    ADD   R2, R2, R5
    LDR   R6, R1, #0         ; load from B → evicts A's line
    ADD   R2, R2, R6
    ADDI  R3, R3, #1

    B     conflict_loop

done:
    HALT
