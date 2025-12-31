/* 内存布局配置 */
ENTRY(_start)

SECTIONS {
    . = 0x80000000;
    
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    
    .rodata : { *(.rodata .rodata.*) }
    .data : { *(.data .data.*) }
    
    /* BSS段 - 需要明确定义开始和结束符号 */
    .bss : {
        _bss_start = .;
        *(.bss .bss.*)
        _bss_end = .;
    }
    
    /* 添加栈空间（在BSS段之后） */
    . = ALIGN(16);
    _stack_bottom = .;
    . += 0x8000;    /* 32KB栈空间 */
    _stack_top = .;

. = ALIGN(8);
_buffer_start = .;
. += 0x100000;   /* 保留1MB空间 */
_buffer_end = .;

}