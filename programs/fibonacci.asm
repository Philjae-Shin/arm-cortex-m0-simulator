; fibonacci.asm — Iterative Fibonacci Benchmark
; Computes Fibonacci(20) iteratively.
; Tests: data dependencies, loop control, register pressure
;
; Algorithm:
;   a = 0, b = 1
;   for i = 2..20:
;       c = a + b
;       a = b
;       b = c
;   result = b

.text
    ; R0 = a (fib[i-2])
    ; R1 = b (fib[i-1])
    ; R2 = c (fib[i])
    ; R3 = counter i
    ; R4 = limit (20)

    MOVI  R0, #0            ; a = 0
    MOVI  R1, #1            ; b = 1
    MOVI  R3, #2            ; i = 2
    MOVI  R4, #21           ; limit (compute fib[20], loop while i < 21)

fib_loop:
    CMP   R3, R4
    BGE   fib_done

    ADD   R2, R0, R1        ; c = a + b
    MOV   R0, R1            ; a = b
    MOV   R1, R2            ; b = c
    ADDI  R3, R3, #1        ; i++

    B     fib_loop

fib_done:
    ; R1 now holds Fibonacci(20) = 6765
    MOV   R10, R1           ; save result in R10
    HALT
