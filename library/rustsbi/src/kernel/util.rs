// library/rustsbi/src/kernel/util.rs
//! 工具函数：打印、格式化等

/// 支持UTF-8的打印字符函数
pub fn print_char(c: char) {
    let mut utf8_buffer = [0u8; 4];
    let utf8_bytes = c.encode_utf8(&mut utf8_buffer).as_bytes();
    
    unsafe {
        let uart = 0x1000_0000 as *mut u8;
        for &byte in utf8_bytes {
            while (uart.add(5).read_volatile() & 0x20) == 0 {}
            uart.write_volatile(byte);
        }
    }
}

/// 打印字符串（支持中文）
pub fn print(s: &str) {
    for c in s.chars() {
        print_char(c);
    }
}
/// 打印十六进制数（字节）
pub fn print_hex(byte: u8) {
    let nibbles = b"0123456789ABCDEF";
    let high = (byte >> 4) as usize;
    let low = (byte & 0x0F) as usize;
    
    unsafe {
        let uart = 0x1000_0000 as *mut u8;
        while (uart.add(5).read_volatile() & 0x20) == 0 {}
        uart.write_volatile(nibbles[high]);
        while (uart.add(5).read_volatile() & 0x20) == 0 {}
        uart.write_volatile(nibbles[low]);
    }
}

/// 打印指针地址（64位）
pub fn print_ptr(ptr: usize) {
    // 打印64位指针地址
    for i in (0..16).rev() {
        let nibble = (ptr >> (i * 4)) as u8 & 0xF;
        let c = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + nibble - 10
        };
        print_char(c as char);
    }
}

/// 打印无符号整数
pub fn print_uint(mut num: u32) {
    let mut buffer = [0u8; 10];
    let mut i = 0;
    
    if num == 0 {
        print("0");
        return;
    }
    
    while num > 0 {
        buffer[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }
    
    for j in (0..i).rev() {
        unsafe {
            let uart = 0x1000_0000 as *mut u8;
            while (uart.add(5).read_volatile() & 0x20) == 0 {}
            uart.write_volatile(buffer[j]);
        }
    }
}

/// 打印32位十六进制数
pub fn print_hex32(value: u32) {
    let nibbles = b"0123456789ABCDEF";
    unsafe {
        let uart = 0x1000_0000 as *mut u8;
        
        for i in (0..8).rev() {
            let nibble = ((value >> (i * 4)) & 0xF) as usize;
            while (uart.add(5).read_volatile() & 0x20) == 0 {}
            uart.write_volatile(nibbles[nibble]);
        }
    }
}

/// 打印64位十六进制数
pub fn print_hex64(value: u64) {
    for i in (0..16).rev() {
        let nibble = (value >> (i * 4)) & 0xF;
        let c = if nibble < 10 {
            (b'0' + nibble as u8) as char
        } else {
            (b'a' + (nibble - 10) as u8) as char
        };
        print_char(c);
    }
}

/// 打印布尔值
pub fn print_bool(b: bool) {
    if b {
        print("true");
    } else {
        print("false");
    }
}

pub fn print_hex16(value: u16) {
    for i in (0..4).rev() {
        let nibble = ((value >> (i * 4)) & 0xF) as u8;
        let c = if nibble < 10 {
            (b'0' + nibble) as char
        } else {
            (b'A' + nibble - 10) as char
        };
        print_char(c);
    }
}