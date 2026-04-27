; vector_ops.asm — Vector Operations Benchmark (SIMD Extension)
; Demonstrates VLDR, VSTR, VADD, VMUL, and VDOT instructions using
; a separate vector register file (V0-V7) for 8-element vectors.
;
; Operations:
;   1. C[i] = A[i] + B[i]  using VADD  (element-wise addition)
;   2. D[i] = A[i] * B[i]  using VMUL  (element-wise multiplication)
;   3. R3   = dot(A, B)     using VDOT  (dot product -> scalar result)
;
; Memory layout:
;   0x1000: A[0..7]  (8 words)
;   0x1020: B[0..7]  (8 words)
;   0x1040: C[0..7]  (result of VADD)
;   0x1060: D[0..7]  (result of VMUL)

.data 0x1000
    arr_a: .word 10, 20, 30, 40, 50, 60, 70, 80
    arr_b: .word 5, 15, 25, 35, 45, 55, 65, 75
    arr_c: .word 0, 0, 0, 0, 0, 0, 0, 0
    arr_d: .word 0, 0, 0, 0, 0, 0, 0, 0

.text
    ; Set up base addresses
    MOVI  R0, #0x1000       ; A base
    MOVI  R1, #0x1020       ; B base
    MOVI  R2, #0x1040       ; C base (VADD result)

    ; Load vectors from memory into vector register file
    VLDR  V0, R0, #8        ; V0 = A[0..7]
    VLDR  V1, R1, #8        ; V1 = B[0..7]

    ; --- Vector Addition: C = A + B ---
    VADD  V2, V0, V1, #8    ; V2[i] = V0[i] + V1[i] for i=0..7
    VSTR  V2, R2, #8        ; Store V2 -> C[0..7]

    ; --- Vector Multiplication: D = A * B ---
    MOVI  R4, #0x1060        ; D base (VMUL result)
    VMUL  V3, V0, V1, #8    ; V3[i] = V0[i] * V1[i] for i=0..7
    VSTR  V3, R4, #8        ; Store V3 -> D[0..7]

    ; --- Dot Product: R3 = dot(A, B) ---
    VDOT  R3, V0, V1, #8    ; R3 = sum(V0[i] * V1[i])

    HALT
