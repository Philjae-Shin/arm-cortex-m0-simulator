; gcd.asm — Greatest Common Divisor (Euclidean Algorithm)
; Computes GCD(252, 105) = 21
; Tests: conditional branches, tight loop, minimal memory usage
;
; Algorithm (subtraction-based):
;   while a != b:
;       if a > b: a = a - b
;       else:     b = b - a
;   result = a

.text
    ; R0 = a
    ; R1 = b
    ; R2 = result

    MOVI  R0, #252          ; a = 252
    MOVI  R1, #105          ; b = 105

gcd_loop:
    CMP   R0, R1
    BEQ   gcd_done          ; a == b (done)

    BGT   a_bigger          ; a > b (subtract b from a)

    ; else: b > a → b = b - a
    SUB   R1, R1, R0
    B     gcd_loop

a_bigger:
    SUB   R0, R0, R1
    B     gcd_loop

gcd_done:
    MOV   R2, R0            ; result = GCD = 21
    HALT
