/* 内存布局配置 - 确保固件加载到正确地址 */
MEMORY {
    /* QEMU virt机器的内存布局 */
    RAM (rwx) : ORIGIN = 0x80200000, LENGTH = 128M
}

SECTIONS {
    /* 入口点设置为0x80200000 */
    . = 0x80200000;
    
    .text : {
        *(.text.entry)   /* 入口代码优先 */
        *(.text .text.*)
    } > RAM
    
    .rodata : { *(.rodata .rodata.*) } > RAM
    .data : { *(.data .data.*) } > RAM  
    .bss : { *(.bss .bss.*) } > RAM
    
    /* 栈指针设置 */
    . = ALIGN(16);
    _stack_start = .;
}