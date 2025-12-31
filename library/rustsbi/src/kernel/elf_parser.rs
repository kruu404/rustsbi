//! ELF file format parser for RISC-V 64-bit

use core::mem;
use crate::kernel::print;

/// ELF magic number
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

/// ELF 64位头（完整标准结构）
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Elf64Ehdr {
    pub e_ident: [u8; 16],     // 0x00-0x0F: ELF标识
    pub e_type: u16,           // 0x10-0x11: 文件类型
    pub e_machine: u16,        // 0x12-0x13: 架构标识
    pub e_version: u32,        // 0x14-0x17: ELF版本
    pub e_entry: u64,          // 0x18-0x1F: 入口点地址
    pub e_phoff: u64,          // 0x20-0x27: 程序头表偏移
    pub e_shoff: u64,          // 0x28-0x2F: 节区头表偏移
    pub e_flags: u32,          // 0x30-0x33: 处理器标志
    pub e_ehsize: u16,         // 0x34-0x35: ELF头大小
    pub e_phentsize: u16,      // 0x36-0x37: 程序头大小
    pub e_phnum: u16,          // 0x38-0x39: 程序头数量
    pub e_shentsize: u16,      // 0x3A-0x3B: 节区头大小
    pub e_shnum: u16,          // 0x3C-0x3D: 节区头数量
    pub e_shstrndx: u16,       // 0x3E-0x3F: 字符串表索引
}

/// ELF 64位程序头（完整版）
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Elf64Phdr {
    pub p_type: u32,           // 段类型
    pub p_flags: u32,          // 段标志
    pub p_offset: u64,         // 段在文件中的偏移
    pub p_vaddr: u64,          // 段的虚拟地址
    pub p_paddr: u64,          // 段的物理地址
    pub p_filesz: u64,         // 段在文件中的长度
    pub p_memsz: u64,          // 段在内存中的长度
    pub p_align: u64,          // 段对齐方式
}

/// 程序头类型常量
const PT_LOAD: u32 = 1;        // 可加载段

/// ELF解析器
pub struct ElfParser<'a> {
    data: &'a [u8],
}

impl<'a> ElfParser<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, &'static str> {
        // 🆕 添加结构体大小验证
    let ehdr_size = core::mem::size_of::<Elf64Ehdr>();
    let phdr_size = core::mem::size_of::<Elf64Phdr>();

    // 必须与 readelf -h 的输出完全一致
    if ehdr_size != 64 {
        return Err("Elf64Ehdr结构体大小与标准不符，存在填充字节或定义错误");
    }
    if phdr_size != 56 {
        return Err("Elf64Phdr结构体大小与标准不符，存在填充字节或定义错误");
    }
        if data.len() < mem::size_of::<Elf64Ehdr>() {
            return Err("ELF文件太小");
        }
        
        // 检查ELF魔数
        if &data[0..4] != ELF_MAGIC {
            return Err("无效的ELF魔数");
        }

        Ok(Self { data })
    }
    
    pub fn entry_point(&self) -> u64 {
        let ehdr = unsafe { &*(self.data.as_ptr() as *const Elf64Ehdr) };
        ehdr.e_entry
    }
    
    /// 完整的段加载实现
    pub fn load_segments<F>(&self, mut load_func: F) -> Result<(), &'static str>
where
    F: FnMut(u64, &[u8], u64),
{
    let ehdr = unsafe { &*(self.data.as_ptr() as *const Elf64Ehdr) };
    
    print("🔍 开始解析程序头表...\r\n");

    // 检查程序头表是否在文件范围内
    let total_phdr_size = (ehdr.e_phnum as usize) * (ehdr.e_phentsize as usize);
    if ehdr.e_phoff as usize + total_phdr_size > self.data.len() {
        print("❌ 程序头表超出文件范围\r\n");
        return Err("程序头表超出文件范围");
    }

    for i in 0..ehdr.e_phnum {
            let phdr_offset = ehdr.e_phoff as usize + (i as usize) * (ehdr.e_phentsize as usize);

            // 🆕 修复：使用 e_phentsize 而不是结构体大小
            if phdr_offset + (ehdr.e_phentsize as usize) > self.data.len() {
                print("❌ 程序头超出文件范围\r\n");
                return Err("程序头超出文件范围");
            }
            
            // 🆕 修复：验证我们读取的数据足够填充 Elf64Phdr 结构
            if phdr_offset + mem::size_of::<Elf64Phdr>() > self.data.len() {
                print("❌ 程序头数据不完整，无法解析\r\n");
                return Err("程序头数据不完整");
            }
            
            let phdr = unsafe { 
                &*((self.data.as_ptr().add(phdr_offset)) as *const Elf64Phdr) 
            };

        // 只处理可加载段
        if phdr.p_type == PT_LOAD {

            // 检查段数据是否在文件范围内
            let file_offset = phdr.p_offset as usize;
            let file_size = phdr.p_filesz as usize;
            
            if file_offset > self.data.len() {
                print("❌ 段文件偏移超出范围\r\n");
                return Err("段文件偏移超出范围");
            }
            
            // 安全计算实际可读数据大小
            let readable_size = if file_offset + file_size > self.data.len() {
                self.data.len() - file_offset  // 调整大小避免越界
            } else {
                file_size
            };
            let segment_data = if readable_size > 0 {
                &self.data[file_offset..file_offset + readable_size]
            } else {
                &[] // 空段（如.bss）
            };

            // 调用加载函数
            load_func(phdr.p_vaddr, segment_data, phdr.p_memsz);
        }
    }
    
    print("🎉 所有段加载完成！\r\n");
    Ok(())
}
    /// Validate ELF file (basic checks)
    pub fn validate(&self) -> Result<(), &'static str> {
        // Basic validation - always pass for now
        Ok(())
    }
}

/// Helper functions for memory operations
pub mod memory {
    use core::ptr;
    pub unsafe fn copy_to_address(dst: *mut u8, src: &[u8]) {
    // 🆕 立即添加：在拷贝前一刻，打印出两个指针的确切值
    use crate::kernel::print;

    // 🆕 关键检查：判断地址是否明显无效
    if (dst as u64) < 0x1000 {
        print("        ❌❌❌ 致命错误：目标地址是非法低地址！拷贝操作已被阻止。\r\n");
        // 可以选择直接返回，或者进入一个安全的状态循环，而不是继续执行导致崩溃。
        loop { /* 安全挂起 */ }
        // 或者 return; 如果您希望跳过此次拷贝
    }
    if (src.as_ptr() as u64) < 0x1000 {
        print("        ❌❌❌ 致命错误：源地址是非法低地址！拷贝操作已被阻止。\r\n");
        loop { /* 安全挂起 */ }
    }

    // 如果地址检查通过，再执行实际的拷贝
    for i in 0..src.len() {
    // 使用 `volatile` 操作防止编译器优化
    core::ptr::write_volatile(dst.add(i) as *mut u8, src[i]);
}
}
    
    /// Zero memory region
    /// # Safety
    /// Caller must ensure the address range is valid and writable
    pub unsafe fn zero_memory(addr: *mut u8, size: usize) {
        ptr::write_bytes(addr, 0, size);
    }

pub unsafe fn load_segment(dst: *mut u8, src: &[u8], memsz: usize) {
    let filesz = src.len();
    use crate::kernel::print;

    if filesz > 0 {
        copy_to_address(dst, src); 
        print("✅ 数据复制完成\r\n");
    }

    // Zero BSS section if memsz > filesz
    if memsz > filesz {
        let bss_size = memsz - filesz;
        let bss_start = dst.add(filesz);
        zero_memory(bss_start, bss_size);
        print("✅ BSS清零完成\r\n");
    }
}
}