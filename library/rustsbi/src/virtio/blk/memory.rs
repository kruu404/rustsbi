// 📄 virtio/blk/memory.rs
//! DMA内存管理和地址分配 - 传统模式
//! 处理传统模式Virtio-blk设备的内存分配

use crate::kernel_loader::{print_uint, print_hex32};
use crate::kernel::print;
use crate::virtio::error::{VirtioError, Result};
use super::device::VirtioBlk;
use crate::kernel::print_hex64;

impl VirtioBlk {
    /// 🛠️ 修复后的传统模式内存分配
   /// 🛠️ 修复后的传统模式内存分配
pub fn allocate_queue_memory(&self, queue_size: u16) -> Result<(u64, u64, u64)> {
    // 🛠️ 关键修复：使用QEMU传统模式固定的内存布局
    let desc_addr = 0x8007_0000u64;  // 描述符表固定地址
    let avail_addr = desc_addr + (queue_size as u64 * 16); // 每个描述符16字节
    let used_addr = 0x8007_1000u64;   // QEMU传统模式固定使用的地址
    // 验证对齐要求
    if desc_addr % 16 != 0 {
        print("❌ Descriptor table not 16-byte aligned\n");
        return Err(VirtioError::DmaError);
    }
    
    if avail_addr % 2 != 0 {
        print("❌ Available ring not 2-byte aligned\n");
        return Err(VirtioError::DmaError);
    }
    
    if used_addr % 4 != 0 {
        print("❌ Used ring not 4-byte aligned\n");
        return Err(VirtioError::DmaError);
    }
    
    // 验证不会内存重叠
    let desc_end = desc_addr + (queue_size as u64 * 16);
    if desc_end > used_addr {
        print("❌ Descriptor table overlaps with Used ring\n");
        return Err(VirtioError::DmaError);
    }
    
    let avail_end = avail_addr + 6 + (queue_size as u64 * 2);
    if avail_end > used_addr {
        print("❌ Available ring overlaps with Used ring\n");
        return Err(VirtioError::DmaError);
    }
    Ok((desc_addr, avail_addr, used_addr))
}
    /// 🆕 传统模式PFN计算（关键！）
    pub fn calculate_legacy_pfn(&self, desc_addr: u64) -> u32 {
        // 🛠️ 传统模式：PFN = 物理地址 / 页大小(4096)
        let pfn = desc_addr / 4096;      
        pfn as u32
    }

    /// 🛠️ 传统模式内核检查（保持原样）
    pub fn check_kernel_at_200(&self) -> bool {
        unsafe {
            let addr = 0x200 as *const u8;
            let byte1 = *addr;
            let byte2 = *addr.add(1);
            let byte3 = *addr.add(2);
            let byte4 = *addr.add(3);
            
            let is_elf = byte1 == 0x7F && byte2 == 0x45 && byte3 == 0x4C && byte4 == 0x46;
            
            if is_elf {
                print("✅ Legacy mode: ELF kernel found at 0x200\r\n");
            } else {
                print("❌ Legacy mode: No kernel at 0x200\r\n");
            }
            
            is_elf
        }
    }

/// 🎯 简单污染检查（只检查关键区域）
    pub fn quick_contamination_check(&self) -> bool {
        let dma_base = 0x80070000u64;
        let mut is_clean = true;
        
        unsafe {
            // 只检查数据缓冲区前16字节
            let buf_ptr = (dma_base + 0x1000) as *const u8;
            for i in 0..16 {
                if *buf_ptr.add(i) != 0 {
                    print("❌ 污染在偏移");
                    print_uint(i as u32);
                    print(": 0x");
                    print_hex32(*buf_ptr.add(i) as u32);
                    print("\r\n");
                    is_clean = false;
                    break;
                }
            }
        }
        
        if is_clean {
            print("✅ 缓冲区干净\r\n");
        } else {
            print("💀 缓冲区已污染！\r\n");
        }
        
        is_clean
    }
    
    /// 🎯 快速清理缓冲区
    pub fn quick_clean_buffer(&self) {
        let dma_base = 0x80070000u64;
        
        unsafe {
            let buf_ptr = (dma_base + 0x1000) as *mut u8;
            for i in 0..512 {
                core::ptr::write_volatile(buf_ptr.add(i), 0);
            }
        }
        
        print("🧹 缓冲区已清理\r\n");
    }

/// 🆕 传统模式内存屏障
    pub fn legacy_memory_barrier(&self) {
        // 传统模式可能需要更强的内存屏障
        unsafe {
            core::arch::asm!("fence iorw, iorw"); // RISC-V完整屏障
        }
        print("✅ Legacy memory barrier executed\r\n");
    }
    
    /// 🆕 传统模式DMA区域验证
    pub fn validate_legacy_dma_region(&self, addr: u64, size: u32) -> bool {
        // 传统模式DMA区域通常为0x80000000-0x88000000
        let valid = addr >= 0x8000_0000 && addr + size as u64 <= 0x8800_0000;
        
        if !valid {
            print("❌ Legacy DMA address out of range: 0x");
            print_hex64(addr);
            print("\r\n");
        }
        
        valid
    }
}