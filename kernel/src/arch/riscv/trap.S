.macro SAVE_REGS, from_user
    addi    sp, sp, -{trapframe_size}
    PUSH_GENERAL_REGS

    csrr    t0, sepc
    csrr    t1, sstatus
    csrrw   t2, sscratch, zero          // save sp and zero scratch
    STR     t0, sp, 31
    STR     t1, sp, 32
    STR     t2, sp, 1

.if \from_user == 1
    LDR     t0, sp, 3                   // load kernel tp
    STR     gp, sp, 2
    STR     tp, sp, 3
    mv      tp, t0
.endif
.endm

.macro RESTORE_REGS, from_user
.if \from_user == 1
    LDR     gp, sp, 2
    LDR     t0, sp, 3
    STR     tp, sp, 3                   // save kernel tp
    mv      tp, t0
    addi    t0, sp, {trapframe_size}    // put kernel sp to scratch
    csrw    sscratch, t0
.endif

    LDR     t0, sp, 31
    LDR     t1, sp, 32
    csrw    sepc, t0
    csrw    sstatus, t1

    POP_GENERAL_REGS
    LDR     sp, sp, 1
.endm

.section .text
.balign 4
trap_vector_base:
    // sscratch == 0: trap from kernel
    // sscratch != 0: trap from user
    csrrw   sp, sscratch, sp
    bnez    sp, .Luser_trap_entry

    // put the stack back
    csrr    sp, sscratch
    j       .Lkernel_trap_entry

.Lkernel_trap_entry:
    SAVE_REGS 0
    mv      a0, sp
    li      a1, 0
    call    riscv_trap_handler
    RESTORE_REGS 0
    sret

.Luser_trap_entry:
    SAVE_REGS 1
    mv      a0, sp
    li      a1, 1
    call    riscv_trap_handler
    RESTORE_REGS 1
    sret
