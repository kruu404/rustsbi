# jump.S
.section .text
.globl jump_to_kernel_asm

jump_to_kernel_asm:
    # 函数签名: void jump_to_kernel_asm(usize entry, usize hartid, usize dtb)
    # 参数寄存器:
    # a0 = 内核入口地址 (entry)
    # a1 = Hart ID (hartid)  
    # a2 = DTB地址 (dtb)
    
    # 1. 保存参数到临时寄存器（避免被后续操作覆盖）
    mv t1, a0   # 保存入口地址到 t1
    mv t2, a1   # 保存 hartid 到 t2
    mv t3, a2   # 保存 dtb 地址到 t3
    
    # 2. 清理浮点状态
    csrc fcsr, zero
    fmv.d.x ft0, zero

    fence iorw, iorw
    
    # 4. 配置 mstatus (MPP=S)
    li t0, 0x1 << 11    # 将1左移11位，设置MPP字段为S-mode (0b01)
    csrw mstatus, t0

    fence iorw, iorw
    
    # 5. 设置 mepc（内核入口点）- 使用传入的参数
    csrw mepc, t1       # 使用传入的入口地址
    
    # 6. PMP配置（允许S-mode访问所有内存）
    li t4, 0xFFFFFFFFFFFFFFFF
    srai t4, t4, 2
    csrw pmpaddr0, t4
    li t5, 0x1F
    csrw pmpcfg0, t5
    
    # 7. 设置内核参数 - 使用传入的参数
    mv a0, t2           # hartid = 传入的hartid
    mv a1, t3           # dtb_addr = 传入的dtb地址
    
    # 8. 跳转到内核
    mret