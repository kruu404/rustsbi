//! Boot environment preparation with enhanced debugging

// 声明外部汇编函数
unsafe extern "C" {
    fn jump_to_kernel_asm(entry: usize, hartid: usize, dtb_addr: usize) -> !;
}

// 现有的打印函数保持不变
fn print_char(c: u8) {
        let uart_base = 0x10000000 as *mut u8;
        while unsafe { uart_base.add(5).read_volatile() } & 0x20 == 0 {}
        unsafe { uart_base.write_volatile(c) };
}

fn print_str(s: &str) {
    for c in s.bytes() {
        print_char(c);
    }
}

fn print_hex(num: usize) {
    let hex_chars = b"0123456789abcdef";
    print_str("0x");
    
    for i in (0..16).rev() {
        let digit = (num >> (i * 4)) & 0xF;
        print_char(hex_chars[digit as usize]);
    }
}

fn print_decimal(num: usize) {
    let mut buffer = [0u8; 20];
    let mut i = 0;
    let mut n = num;
    
    if n == 0 {
        print_char(b'0');
        return;
    }
    
    while n > 0 {
        buffer[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    
    for j in (0..i).rev() {
        print_char(buffer[j]);
    }
}

/// Rust包装函数 - 修正版本
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jump_to_kernel(entry: usize, hartid: usize, dtb: usize) -> ! {
    // 添加调试信息
    print_str("\r\n🔍 === 引导参数验证 ===\r\n");
    print_str("内核入口地址： ");
    print_hex(entry);
    print_str("\r\n硬件线程ID： ");
    print_decimal(hartid);
    print_str("\r\n设备树地址： ");
    print_hex(dtb);
    print_str("\r\n================================\r\n\r\n");
    
    // 直接调用汇编函数
    jump_to_kernel_asm(entry, hartid, dtb);
}

/// Complete boot process
pub fn boot_kernel(entry: usize, hartid: usize, dtb_addr: usize) -> ! {
    print_str("\r\n🚀 内核引导阶段开始...\r\n");
    
    unsafe {
        jump_to_kernel(entry, hartid, dtb_addr);
    }
}