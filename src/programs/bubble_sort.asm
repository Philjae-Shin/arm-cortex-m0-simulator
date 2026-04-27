; bubble_sort.asm — Bubble Sort Benchmark
; Sorts an array of 8 integers in ascending order.
; Tests: nested loops, conditional branches, memory access patterns
;
; Memory layout:
;   0x2000: array[0..7]

.data 0x2000
    array: .word 42, 17, 93, 5, 28, 64, 11, 76

.text
    ; R0 = array base address
    ; R1 = n (array length)
    ; R2 = outer loop counter i
    ; R3 = inner loop counter j
    ; R4 = inner limit (n - i - 1)
    ; R5 = arr[j]
    ; R6 = arr[j+1]
    ; R7 = temp address for arr[j]
    ; R8 = temp address for arr[j+1]

    MOVI  R0, #0x2000       ; array base
    MOVI  R1, #8            ; n = 8
    MOVI  R2, #0            ; i = 0

outer:
    SUB   R4, R1, R2        ; R4 = n - i
    SUBI  R4, R4, #1        ; R4 = n - i - 1
    CMPI  R4, #1            ; if n-i-1 < 1, sorted
    BLT   sorted
    MOVI  R3, #0            ; j = 0

inner:
    CMP   R3, R4
    BGE   next_pass

    ; Compute addresses of arr[j] and arr[j+1]
    LSL   R7, R3, #2        ; R7 = j * 4
    ADD   R7, R0, R7        ; R7 = &arr[j]

    LDR   R5, [R7, #0]     ; R5 = arr[j]
    LDR   R6, [R7, #4]     ; R6 = arr[j+1]

    CMP   R5, R6            ; if arr[j] <= arr[j+1], no swap
    BLE   no_swap

    ; Swap arr[j] and arr[j+1]
    STR   R6, [R7, #0]     ; arr[j] = arr[j+1]
    STR   R5, [R7, #4]     ; arr[j+1] = arr[j]

no_swap:
    ADDI  R3, R3, #1       ; j++
    B     inner

next_pass:
    ADDI  R2, R2, #1       ; i++
    B     outer

sorted:
    HALT
