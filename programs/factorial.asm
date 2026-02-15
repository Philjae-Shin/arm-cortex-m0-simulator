; factorial.asm — Recursive Factorial Benchmark
; Computes factorial(10) = 3628800 using recursive function calls.
; Tests: function call/return, PUSH/POP, link register, recursion
;
; Pseudo-code:
;   int factorial(int n) {
;       if (n <= 1) return 1;
;       return n * factorial(n - 1);
;   }

.text
    ; Set up initial call: factorial(10)
    MOVI  R0, #10           ; argument: n = 10
    BL    factorial          ; call factorial(10)
    ; R0 now holds result = 3628800
    MOV   R10, R0           ; save result
    HALT

; ---- factorial(n) ----
; Input:  R0 = n
; Output: R0 = n!
factorial:
    PUSH  {R4, LR}          ; save callee-saved reg and return address

    CMPI  R0, #1             ; if n <= 1
    BLE   base_case

    MOV   R4, R0             ; R4 = n (save for later multiply)
    SUBI  R0, R0, #1         ; R0 = n - 1
    BL    factorial           ; R0 = factorial(n - 1)
    MUL   R0, R4, R0         ; R0 = n * factorial(n - 1)

    POP   {R4, LR}          ; restore
    BX    LR                 ; return

base_case:
    MOVI  R0, #1             ; return 1
    POP   {R4, LR}          ; restore
    BX    LR                 ; return
