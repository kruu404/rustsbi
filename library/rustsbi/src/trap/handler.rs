// library/rustsbi/src/trap/handler.rs
use core::arch::asm;
use crate::kernel::print;
use crate::kernel::print_char;
use crate::kernel::print_hex64;

/// 直接基于CSR读取的陷阱处理函数
#[unsafe(no_mangle)]
pub extern "C" fn trap_handler() -> u64 {
    // 直接读取所有关键CSR寄存器
    let (mcause, mepc, mtval, mstatus, a0, a1, a6, a7): (u64, u64, u64, u64, u64, u64, u64, u64);
    
    unsafe {
        asm!(
            // 读取CSR寄存器
            "csrr {0}, mcause",
            "csrr {1}, mepc", 
            "csrr {2}, mtval",
            "csrr {3}, mstatus",
            // 读取通用寄存器参数
            "mv {4}, a0",
            "mv {5}, a1",
            "mv {6}, a6",
            "mv {7}, a7",
            out(reg) mcause,
            out(reg) mepc,
            out(reg) mtval,
            out(reg) mstatus,
            out(reg) a0,
            out(reg) a1,
            out(reg) a6,
            out(reg) a7,
        );
    }
    
    // 打印真实的陷阱信息
  //  print_direct_trap_info(mcause, mepc, mtval, mstatus, a7, a6, a0, a1);
    
    // 根据mcause进行分发处理
    match mcause & 0x7FFF_FFFF {
        0x9 => { // Environment call from S-mode
            handle_sbi_call_direct(a7, a6, a0, a1, mepc)
        }
        0xb => { // Environment call from M-mode
            handle_mmode_ecall_direct(mepc)
        }
        _ => {
            handle_unknown_trap_direct(mcause, mepc)
        }
    }
}

/// 处理 SBI 调用 (直接版本)
fn handle_sbi_call_direct(extension_id: u64, function_id: u64, arg0: u64, arg1: u64, mepc: u64) -> u64 {
    // 处理不同的SBI扩展[1,4](@ref)
    let (error, value) = match extension_id {
        0x00 => { // 基础扩展 (Base Extension)
            handle_base_extension(function_id, arg0, arg1)
        }
        0x01 => { // 控制台扩展 (Console Extension) - 关键修复
            handle_console_extension(function_id, arg0, arg1)
        }
        0x53525354 => { // "SRST" - 系统关机
            print("\r\n🔌 收到关机请求\r\n");
            shutdown();
        }
        0x54494D45 => { // "TIME" - 定时器扩展
            handle_timer_extension(function_id, arg0, arg1)
        }
        0x444E4942 => { // "BIND" - 厂商特定扩展
            handle_vendor_extension(function_id, arg0, arg1)
        }
        _ => {
            print("⚠️ Unknown SBI extension: 0x");
            print_hex64(extension_id);
            print("\r\n");
            (0xFFFFFFFFFFFFFFFF, 0) // 错误码[4](@ref)
        }
    };
    
    // 设置返回值到寄存器[1,4](@ref)
    unsafe {
        asm!(
            "mv a0, {0}",
            "mv a1, {1}",
            in(reg) error,
            in(reg) value
        );
    }
    
    // 跳过ecall指令 (4字节)
    mepc + 4
}

/// 处理控制台扩展 - 新增函数
fn handle_console_extension(function_id: u64, arg0: u64, _arg1: u64) -> (u64, u64) {
    match function_id {
        0x00 => { // 控制台输出字符
            let ch = (arg0 & 0xFF) as u8;
            print_char(ch as char);
            (0, 0) // 成功
        }
        0x01 => { // 控制台读取字符
            // 简单实现：返回无输入
            (0, 0xFFFFFFFFFFFFFFFF)
        }
        _ => {
            (0xFFFFFFFFFFFFFFFF, 0) // 不支持的函数
        }
    }
}

/// 处理基础SBI扩展[1](@ref)
fn handle_base_extension(function_id: u64, arg0: u64, _arg1: u64) -> (u64, u64) {
    match function_id {
        0x00 => { // 获取SBI规范版本
            // 返回一个示例版本号，如0.2
            (0, 0x00000002)
        }
        0x01 => { // 获取SBI实现ID
            // 返回您的实现ID，如自定义值
            (0, 0x52535342) // "RSSB"
        }
        0x02 => { // 获取SBI实现版本
            (0, 0x00000001) // 版本1.0
        }
        _ => {
            (0xFFFFFFFFFFFFFFFF, 0) // 不支持的函数
        }
    }
}

/// 处理定时器扩展
fn handle_timer_extension(function_id: u64, arg0: u64, arg1: u64) -> (u64, u64) {
    match function_id {
        0x00 => { // 设置定时器
            print("⏰ Timer set requested\r\n");
            // 这里可以添加实际的定时器设置逻辑
            (0, 0) // 成功
        }
        _ => {
            (0xFFFFFFFFFFFFFFFF, 0) // 不支持的函数
        }
    }
}

/// 处理厂商特定扩展
fn handle_vendor_extension(_function_id: u64, _arg0: u64, _arg1: u64) -> (u64, u64) {
    // 暂时不实现厂商特定功能
    (0xFFFFFFFFFFFFFFFF, 0)
}

/// 处理 M 模式 ecall
fn handle_mmode_ecall_direct(mepc: u64) -> u64 {
    print("⚠️ M-mode ecall detected\r\n");
    mepc + 4
}

/// 处理未知陷阱
fn handle_unknown_trap_direct(mcause: u64, mepc: u64) -> u64 {
    print("❌ Unknown trap detected: mcause=0x");
    print_hex64(mcause);
    print("\r\n");
    
    // 尝试跳过当前指令，或者进入安全处理
    if (mcause & 0x7FFF_FFFF) == 0x1 {
        print("🚨 Instruction access fault - attempting recovery\r\n");
        mepc + 4 // 跳过故障指令
    } else {
        // 严重错误，进入关机流程
        shutdown();
    }
}

/// 安全关机函数[5](@ref)
fn shutdown() -> ! {
    print("🛑 安全关机...\r\n");
    
    unsafe {
        // QEMU Virt 平台的关机机制
        let test_fdt_addr = 0x100000 as *mut u32;
        test_fdt_addr.write_volatile(0x5555); // QEMU 关机魔法值
    }
    
    // 无限等待
    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}