// examples/minimal_boot.rs
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use rustsbi::KernelError;
use rustsbi::kernel;
use rustsbi::kernel::elf_parser::{ElfParser, memory};
use rustsbi::kernel::boot_env;

// 从链接脚本引入符号
unsafe extern "C" {
    static _bss_start: u8;
    static _bss_end: u8;
    static _stack_top: u8;
}

#[unsafe(no_mangle)] 
pub extern "C" fn main() -> ! {

print("⏳ 等待硬件稳定...\r\n");
    wait_for_hardware_stability();

    clear_bss();

    print("\r\n=== RISC-V 系统引导开始 ===\r\n");
    
    match kernel::create_kernel_loader() {
        Ok(mut loader) => {
            match loader.find_and_load_kernel() {
                Ok(()) => {
                    // 🛠️ 关键修改：使用新的方法获取缓冲区切片
        let (buffer_slice, _elf_offset) = loader.get_elf_data_with_offset();
                    
                    // 搜索ELF签名位置
                    let elf_offset = find_elf_signature(buffer_slice);

                    match elf_offset {
                        Some(offset) => {
                            print("🎯 找到ELF签名！");
                            print("\r\n");
                            
                            // 从正确的位置创建ELF解析器
                            let elf_data = &buffer_slice[offset..];
                            let _elf_parser = match ElfParser::new(elf_data) {
                                Ok(parser) => {
                                    print("✅ ELF文件解析成功\r\n");
                                    
                                    // 验证入口点
                                    let entry_point = parser.entry_point();
                                    // 验证入口点合理性
                                    if !is_valid_entry_point(entry_point) {
                                        print("⚠️ 入口点地址异常，使用默认地址 0x80400000\r\n");
                                        jump_to_kernel(0x80400000);
                                    }
                                    
                                    // 加载段到内存
                                    print("💾 加载段到内存...\r\n");
                                    if let Err(e) = parser.load_segments(|vaddr, data, memsz| {                                       
                                        unsafe {
                                            memory::load_segment(vaddr as *mut u8, data, memsz as usize);
                                        }
                                    }) {
                                        print("⚠️ 段加载警告: ");
                                        print(e);
                                        print("\r\n");
                                    }
                                    
                                    print("✅ 内核加载完成，准备跳转...\r\n");
                                    jump_to_kernel(entry_point);
                                }
                                Err(e) => {
                                    print("❌ ELF解析失败: ");
                                    print(e);
                                    print("\r\n");
                                    panic_with_message("ELF解析失败");
                                }
                            };
                        }
                        None => {
                            print("❌ 未找到ELF签名\r\n");
                            panic_with_message("没有有效的内核文件");
                        }
                    }
                }
                Err(e) => {
                    print("❌ 内核加载失败: ");
                    match e {
                        KernelError::InitFailed => print("设备初始化失败\r\n"),
                        KernelError::IoError => print("磁盘读取错误\r\n"),
                        KernelError::BufferTooSmall => print("缓冲区太小\r\n"),
                        KernelError::DeviceNotFound => print("设备未找到\r\n"),
                        _ => print("未知错误\r\n"),
                    }
                    safe_shutdown();
                }
            }
        }
        Err(e) => {
            print("❌ 创建加载器失败: ");
            match e {
                KernelError::DeviceNotFound => print("未找到Virtio设备\r\n"),
                _ => print("未知错误\r\n"),
            }
            safe_shutdown();
        }
    }
}

/// 检查入口点是否合理
fn is_valid_entry_point(entry_point: u64) -> bool {
    // RISC-V内核标准入口点范围
    entry_point >= 0x80000000 && entry_point < 0x90000000
}

fn wait_for_hardware_stability() {
    // 简单的软件延迟循环
    // 根据您的CPU频率调整延迟计数
    const DELAY_COUNT: u32 = 1000_000_000; 
    unsafe {
        for _ in 0..DELAY_COUNT {
            core::arch::asm!("nop"); // 无操作指令，产生延迟
        }
    }
    
    print("✅ 硬件稳定等待完成\r\n");
}

/// 直接验证缓冲区开头的ELF签名
fn find_elf_signature(data: &[u8]) -> Option<usize> {
    // 确保数据长度足够包含ELF魔数
    if data.len() < 4 {
        print("❌ 缓冲区数据不足，无法验证ELF签名\r\n");
        return None;
    }
    
    // 直接检查前4个字节是否为ELF签名
    if data[0] == 0x7F && data[1] == b'E' && data[2] == b'L' && data[3] == b'F' {
        
        return Some(0); // 总是返回偏移量0
    } else {
        // 详细显示前4个字节的内容用于调试
        print("❌ 缓冲区开头不是有效的ELF签名\r\n");
        print("🔍 前4个字节: ");
        print_hex32(data[0] as u32);
        print(" ");
        print_hex32(data[1] as u32);
        print(" ");
        print_hex32(data[2] as u32);
        print(" ");
        print_hex32(data[3] as u32);
        print("\r\n");
        print("   期望: 7F 45 4C 46 (0x7F 'E' 'L' 'F')\r\n");
        
        return None;
    }
}

/// 带消息的panic函数
fn panic_with_message(message: &str) -> ! {
    print("\r\n💥 PANIC! ");
    print(message);
    print("\r\n");
    loop {
        unsafe { core::arch::asm!("nop"); }
    }
}

/// 安全关机
fn safe_shutdown() -> ! {
    print("🔴 系统安全关闭...\r\n");
    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}

fn jump_to_kernel(entry_point: u64) -> ! {
    let hartid = 0;
    let dtb_addr = 0x87000000;
    boot_env::boot_kernel(
            entry_point as usize, 
            hartid, 
            dtb_addr
        );
}

// 串口输出函数（保持不变）
fn print(s: &str) {
    for &byte in s.as_bytes() {
        if byte == b'\n' {
            print_char('\r');
        }
        print_char(byte as char);
    }
}

fn print_char(c: char) {
    unsafe {
        let uart = 0x1000_0000 as *mut u8;
        while (uart.add(5).read_volatile() & 0x20) == 0 {}
        uart.write_volatile(c as u8);
    }
}

fn print_hex32(value: u32) {
    for i in (0..8).rev() {
        let nibble = (value >> (i * 4)) as u8 & 0xF;
        let c = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + nibble - 10
        };
        print_char(c as char);
    }
}

fn clear_bss() {
    unsafe {
        let bss_start = &_bss_start as *const u8 as usize;
        let bss_end = &_bss_end as *const u8 as usize;
        let bss_size = bss_end - bss_start;
        if bss_size > 0 {
            core::ptr::write_bytes(bss_start as *mut u8, 0, bss_size);
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print("\r\n💥 PANIC! 系统崩溃\r\n");
    loop {}
}