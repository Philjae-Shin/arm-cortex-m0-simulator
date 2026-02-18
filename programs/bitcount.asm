; bitcount.asm — Population Count (Hamming Weight) Benchmark
; Counts the number of set bits in a 32-bit value.
; Tests: tight loop, bit manipulation, shift operations
;
; Algorithm:
;   count = 0
;   while val != 0:
;       count += val & 1
;       val >>= 1

.text
    ; R0 = value to count bits of
    ; R1 = bit counter
    ; R2 = temp (val & 1)
    ; R3 = constant 1

    MOVI  R0, #0x5555       ; val = 0x55555555 (16 bits set in lower half)
    ; To set upper bits, use shift and OR:
    LSL   R4, R0, #16       ; R4 = 0x55550000
    ORR   R0, R0, R4        ; R0 = 0x55555555  (16 bits set)

    MOVI  R1, #0            ; count = 0
    MOVI  R3, #1            ; constant 1

count_loop:
    CMPI  R0, #0            ; if val == 0, done
    BEQ   count_done

    AND   R2, R0, R3        ; R2 = val & 1
    ADD   R1, R1, R2        ; count += bit
    LSR   R0, R0, #1        ; val >>= 1

    B     count_loop

count_done:
    ; R1 = 16 (number of set bits in 0x55555555)
    MOV   R10, R1           ; save result
    HALT
