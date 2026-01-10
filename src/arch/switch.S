# RISC-V 64-bit Context Switching Assembly
# INTEGER-ONLY VERSION (No Floating Point)
# Use this version for rv64imac target

.section .text

# =============================================================================
# perform_context_switch - Context switch without floating-point
# =============================================================================
# Arguments:
#   a0 (x10) = from_tcb pointer (can be null for first task)
#   a1 (x11) = to_tcb pointer
#
# This function saves the current task's integer registers and loads the
# next task's integer registers. It does NOT save floating-point registers.

.global perform_context_switch
perform_context_switch:
    # Check if from_tcb is null
    beqz    a0, load_new_context_no_fp
    
    # =========================================================================
    # SAVE CURRENT CONTEXT (Integer registers only - 248 bytes)
    # =========================================================================
    
    # Allocate space on stack for context (31 registers × 8 bytes = 248 bytes)
    addi    sp, sp, -248
    
    # Save all integer registers (x1-x31) except x0 (which is always 0)
    sd      x1,   0(sp)    # ra  - Return address
    sd      x2,   8(sp)    # sp  - Stack pointer (old value)
    sd      x3,  16(sp)    # gp  - Global pointer
    sd      x4,  24(sp)    # tp  - Thread pointer
    sd      x5,  32(sp)    # t0  - Temporary 0
    sd      x6,  40(sp)    # t1  - Temporary 1
    sd      x7,  48(sp)    # t2  - Temporary 2
    sd      x8,  56(sp)    # s0  - Saved 0 / Frame pointer
    sd      x9,  64(sp)    # s1  - Saved 1
    sd      x10, 72(sp)    # a0  - Argument 0 / Return value 0
    sd      x11, 80(sp)    # a1  - Argument 1 / Return value 1
    sd      x12, 88(sp)    # a2  - Argument 2
    sd      x13, 96(sp)    # a3  - Argument 3
    sd      x14, 104(sp)   # a4  - Argument 4
    sd      x15, 112(sp)   # a5  - Argument 5
    sd      x16, 120(sp)   # a6  - Argument 6
    sd      x17, 128(sp)   # a7  - Argument 7
    sd      x18, 136(sp)   # s2  - Saved 2
    sd      x19, 144(sp)   # s3  - Saved 3
    sd      x20, 152(sp)   # s4  - Saved 4
    sd      x21, 160(sp)   # s5  - Saved 5
    sd      x22, 168(sp)   # s6  - Saved 6
    sd      x23, 176(sp)   # s7  - Saved 7
    sd      x24, 184(sp)   # s8  - Saved 8
    sd      x25, 192(sp)   # s9  - Saved 9
    sd      x26, 200(sp)   # s10 - Saved 10
    sd      x27, 208(sp)   # s11 - Saved 11
    sd      x28, 216(sp)   # t3  - Temporary 3
    sd      x29, 224(sp)   # t4  - Temporary 4
    sd      x30, 232(sp)   # t5  - Temporary 5
    sd      x31, 240(sp)   # t6  - Temporary 6
    
    # Update from_tcb->stack_top with current SP
    # TCB structure: stack_top is at offset 0 (FIRST FIELD!)
    sd      sp, 0(a0)      # from_tcb->stack_top = sp
    
load_new_context_no_fp:
    # =========================================================================
    # LOAD NEW CONTEXT
    # =========================================================================
    
    # Load new task's SP from to_tcb->stack_top
    # TCB structure: stack_top is at offset 0
    ld      sp, 0(a1)      # sp = to_tcb->stack_top
    
    # Restore all integer registers from stack
    # Load them in the same order we saved them
    
    ld      x1,   0(sp)    # ra  - Return address
    # Skip x2 (sp) for now, will restore at end
    ld      x3,  16(sp)    # gp  - Global pointer
    ld      x4,  24(sp)    # tp  - Thread pointer
    ld      x5,  32(sp)    # t0  - Temporary 0
    ld      x6,  40(sp)    # t1  - Temporary 1
    ld      x7,  48(sp)    # t2  - Temporary 2
    ld      x8,  56(sp)    # s0  - Saved 0 / Frame pointer
    ld      x9,  64(sp)    # s1  - Saved 1
    ld      x10, 72(sp)    # a0  - Argument 0
    ld      x11, 80(sp)    # a1  - Argument 1
    ld      x12, 88(sp)    # a2  - Argument 2
    ld      x13, 96(sp)    # a3  - Argument 3
    ld      x14, 104(sp)   # a4  - Argument 4
    ld      x15, 112(sp)   # a5  - Argument 5
    ld      x16, 120(sp)   # a6  - Argument 6
    ld      x17, 128(sp)   # a7  - Argument 7
    ld      x18, 136(sp)   # s2  - Saved 2
    ld      x19, 144(sp)   # s3  - Saved 3
    ld      x20, 152(sp)   # s4  - Saved 4
    ld      x21, 160(sp)   # s5  - Saved 5
    ld      x22, 168(sp)   # s6  - Saved 6
    ld      x23, 176(sp)   # s7  - Saved 7
    ld      x24, 184(sp)   # s8  - Saved 8
    ld      x25, 192(sp)   # s9  - Saved 9
    ld      x26, 200(sp)   # s10 - Saved 10
    ld      x27, 208(sp)   # s11 - Saved 11
    ld      x28, 216(sp)   # t3  - Temporary 3
    ld      x29, 224(sp)   # t4  - Temporary 4
    ld      x30, 232(sp)   # t5  - Temporary 5
    ld      x31, 240(sp)   # t6  - Temporary 6
    
    # Restore stack pointer (deallocate context)
    addi    sp, sp, 248
    
    # Jump to restored task's return address (ra)
    # This will either:
    #   - Return to where the task was interrupted (if it was switched out before)
    #   - Jump to task entry point (if this is the task's first run)
    ret

# =============================================================================
# restore_context - Restore context and start executing (for first task)
# =============================================================================
# Arguments:
#   a0 (x10) = stack pointer
#
# This function restores a task's context and starts it executing.
# Used for starting the first task. Never returns.

.global restore_context
restore_context:
    # Set stack pointer to provided value
    mv      sp, a0
    
    # Restore all integer registers (same as load_new_context_no_fp)
    ld      x1,   0(sp)    # ra
    ld      x3,  16(sp)    # gp
    ld      x4,  24(sp)    # tp
    ld      x5,  32(sp)    # t0
    ld      x6,  40(sp)    # t1
    ld      x7,  48(sp)    # t2
    ld      x8,  56(sp)    # s0
    ld      x9,  64(sp)    # s1
    ld      x10, 72(sp)    # a0
    ld      x11, 80(sp)    # a1
    ld      x12, 88(sp)    # a2
    ld      x13, 96(sp)    # a3
    ld      x14, 104(sp)   # a4
    ld      x15, 112(sp)   # a5
    ld      x16, 120(sp)   # a6
    ld      x17, 128(sp)   # a7
    ld      x18, 136(sp)   # s2
    ld      x19, 144(sp)   # s3
    ld      x20, 152(sp)   # s4
    ld      x21, 160(sp)   # s5
    ld      x22, 168(sp)   # s6
    ld      x23, 176(sp)   # s7
    ld      x24, 184(sp)   # s8
    ld      x25, 192(sp)   # s9
    ld      x26, 200(sp)   # s10
    ld      x27, 208(sp)   # s11
    ld      x28, 216(sp)   # t3
    ld      x29, 224(sp)   # t4
    ld      x30, 232(sp)   # t5
    ld      x31, 240(sp)   # t6
    
    # Restore stack pointer
    addi    sp, sp, 248
    
    # Jump to task entry point (stored in ra)
    ret