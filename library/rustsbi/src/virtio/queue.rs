// library/rustsbi/src/virtio/queue.rs
#![allow(dead_code)]

use core::ptr;
use crate::virtio::error::{VirtioError, Result};
use crate::kernel_loader::{print, print_uint, print_hex32, print_hex64};

/// Virtqueue描述符 - 强制16字节对齐
#[repr(C, align(16))]
pub struct Descriptor {
    pub addr: u64,    // 物理地址
    pub len: u32,     // 缓冲区长度
    pub flags: u16,   // 描述符标志
    pub next: u16,    // 下一个描述符索引
}

/// 可用环结构
#[repr(C)]
pub struct AvailableRing {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; 256],
    // 注意：传统模式没有used_event字段
}

/// 已用环结构
#[repr(C)]
pub struct UsedRing {
    pub flags: u16,
    pub idx: u16,
    pub ring: [UsedElem; 256],
    // 注意：传统模式没有avail_event字段
}

/// 已用环元素
#[repr(C)]
#[derive(Copy, Clone)]
pub struct UsedElem {
    pub id: u32,    // 描述符索引
    pub len: u32,   // 写入的数据长度
}

/// Virtqueue核心结构
pub struct Virtqueue {
    pub desc: *mut Descriptor,      // 描述符表
    pub avail: *mut AvailableRing, // 可用环
    pub used: *mut UsedRing,       // 已用环
    pub queue_size: u16,           // 队列大小
    pub free_head: u16,            // 空闲描述符头
    pub num_free: u16,             // 空闲描述符数量
    pub last_used_idx: u16,        // 最后使用的索引
    pub desc_size: usize,
}

impl Virtqueue {
    /// 创建新的Virtqueue - 修复版
    pub fn new(desc_addr: usize, avail_addr: usize, used_addr: usize, size: u16) -> Result<Self> {
        
        if size == 0 || size > 1024 {
            print("❌ Invalid size\r\n");
            return Err(VirtioError::InvalidParam);
        }
        
        // 🛠️ 关键修复：验证地址对齐
        if desc_addr % 16 != 0 {
            print("❌ Descriptor table not 16-byte aligned! addr=0x");
            print_hex32(desc_addr as u32);
            print("\r\n");
            return Err(VirtioError::MemoryNotAligned);
        }
        
        if avail_addr % 2 != 0 {
            print("❌ Available ring not 2-byte aligned! addr=0x");
            print_hex32(avail_addr as u32);
            print("\r\n");
            return Err(VirtioError::MemoryNotAligned);
        }
        
        if used_addr % 4 != 0 {
            print("❌ Used ring not 4-byte aligned! addr=0x");
            print_hex32(used_addr as u32);
            print("\r\n");
            return Err(VirtioError::MemoryNotAligned);
        }
        
        // 🛠️ 关键修复：验证内存布局
        Self::validate_memory_layout(desc_addr, avail_addr, used_addr, size)?;
        
        unsafe {
            let desc = desc_addr as *mut Descriptor;
            let avail = avail_addr as *mut AvailableRing;
            let used = used_addr as *mut UsedRing;
            
            // 🛠️ 修复：正确的描述符初始化（使用固定16字节大小）
            for i in 0..size {
                let desc_ptr = desc.byte_offset(i as isize * 16); // 固定16字节
                
                (*desc_ptr).addr = 0u64;
                (*desc_ptr).len = 0u32;
                (*desc_ptr).flags = 0u16;
                (*desc_ptr).next = if i == size - 1 { 
                    0u16  // 最后一个描述符
                } else { 
                    (i + 1) as u16  // 指向下一个
                };
            }
            
            // 初始化可用环
            (*avail).flags = 0u16;
            (*avail).idx = 0u16;
            
            // 初始化已用环  
            (*used).flags = 0u16;
            (*used).idx = 0u16;
            
            let vq = Virtqueue {
                desc,
                avail,
                used,
                queue_size: size,
                free_head: 0,
                num_free: size,
                last_used_idx: 0,
		desc_size: 16,
            };
            Ok(vq)
        }
    }
    
    /// 🆕 验证内存布局 - 完全重写
fn validate_memory_layout(desc_addr: usize, avail_addr: usize, used_addr: usize, queue_size: u16) -> Result<()> {
    // 🛠️ 关键修复：QEMU传统模式固定布局
    let expected_desc_addr = 0x80070000usize;
    let expected_avail_addr = expected_desc_addr + (16 * queue_size as usize);
    let expected_used_addr = 0x80071000usize; // QEMU固定地址
    
    // 🛠️ 关键验证：必须与QEMU期望完全匹配
    if desc_addr != expected_desc_addr {
        print("❌ CRITICAL: Descriptor address mismatch!\r\n");
        print("   QEMU expects: 0x"); print_hex32(expected_desc_addr as u32); print("\r\n");
        print("   Driver set: 0x"); print_hex32(desc_addr as u32); print("\r\n");
        return Err(VirtioError::MemoryNotAligned);
    }
    
    if avail_addr != expected_avail_addr {
        print("❌ CRITICAL: Available ring address mismatch!\r\n");
        print("   Expected after desc: 0x"); print_hex32(expected_avail_addr as u32); print("\r\n");
        print("   Actual: 0x"); print_hex32(avail_addr as u32); print("\r\n");
        return Err(VirtioError::MemoryNotAligned);
    }
    
    // 🛠️ 最关键修复：Used Ring必须严格匹配QEMU的0x80071000
    if used_addr != expected_used_addr {
        print("❌ CRITICAL: Used ring address mismatch - THIS IS THE MAIN ISSUE!\r\n");
        print("   QEMU FIXED EXPECTATION: 0x"); print_hex32(expected_used_addr as u32); print("\r\n");
        print("   Driver provided: 0x"); print_hex32(used_addr as u32); print("\r\n");
        print("   This explains why used.idx updates are not visible!\r\n");
        return Err(VirtioError::MemoryNotAligned);
    }
    
    // 验证对齐要求（根据Virtio规范）
    if desc_addr % 16 != 0 {
        print("❌ Descriptor table not 16-byte aligned!\r\n");
        return Err(VirtioError::MemoryNotAligned);
    }
    
    if avail_addr % 2 != 0 {
        print("❌ Available ring not 2-byte aligned!\r\n");
        return Err(VirtioError::MemoryNotAligned);
    }
    
    if used_addr % 4 != 0 {
        print("❌ Used ring not 4-byte aligned!\r\n");
        return Err(VirtioError::MemoryNotAligned);
    }
    
    // 验证内存不重叠
    let desc_end = desc_addr + (16 * queue_size as usize);
    if desc_end > avail_addr {
        print("❌ Descriptor table overlaps with available ring!\r\n");
        return Err(VirtioError::MemoryNotAligned);
    }
    
    let avail_end = avail_addr + 6 + (2 * queue_size as usize);
    if avail_end > used_addr {
        print("❌ Available ring overlaps with used ring!\r\n");
        return Err(VirtioError::MemoryNotAligned);
    }

    Ok(())
}

    /// 🆕 安全的描述符指针获取方法 - 修复版
    fn get_descriptor_ptr(&self, index: u16) -> Result<*mut Descriptor> {
        if index >= self.queue_size {
            return Err(VirtioError::InvalidParam);
        }
        
        // 🛠️ 修复：使用固定16字节偏移
        let ptr = unsafe { 
            self.desc.byte_offset(index as isize * 16) // 固定16字节
        };
        
        if ptr.is_null() {
            return Err(VirtioError::DmaError);
        }
        
        Ok(ptr)
    }

    /// 分配描述符链 - 修复版
    pub fn alloc_desc_chain(&mut self, num: u16) -> Result<u16> {
        print("     [alloc_desc_chain] Requesting ");
        print_uint(num as u32);
        print(" descriptors... ");
        
        if num == 0 || num > self.queue_size || self.num_free < num {
            print("❌ Invalid parameters\r\n");
            return Err(VirtioError::InvalidParam);
        }
        
        let head = self.free_head;
        let mut current = head;
        
        for i in 0..num {
            if let Ok(desc_ptr) = self.get_descriptor_ptr(current) {
                unsafe {
                    if i == num - 1 {
                        // 最后一个描述符，next=0
                        (*desc_ptr).next = 0u16;
                    } else {
                        // 指向下一个描述符
                        (*desc_ptr).next = (current + 1) as u16;
                        current = current + 1;
                    }
                }
            } else {
                print("❌ Failed to get descriptor pointer for index ");
                print_uint(current as u32);
                print("\r\n");
                return Err(VirtioError::DmaError);
            }
        }
        
        // 更新空闲链表头
        self.free_head = (current + 1) % self.queue_size;
        self.num_free -= num;
        
        Ok(head)
    }
    
    /// 设置描述符 - 修复版
    pub fn set_descriptor(&mut self, index: u16, addr: u64, len: u32, flags: u16, next: u16) -> Result<()> {
        // 🛠️ 关键修复：传统模式使用原生字节序
        if let Ok(desc_ptr) = self.get_descriptor_ptr(index) {
            unsafe {
                // 🛠️ 修复：传统模式不使用小端转换
                (*desc_ptr).addr = addr;  // 原生字节序
                (*desc_ptr).len = len;    // 原生字节序
                (*desc_ptr).flags = flags; // 原生字节序
                (*desc_ptr).next = next;  // 原生字节序
                
                // 内存屏障
                core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
            }
            Ok(())
        } else {
            print("   ❌ Invalid descriptor index\r\n");
            Err(VirtioError::InvalidParam)
        }
    }

    /// 将描述符添加到可用环 - 修复版
    pub fn add_to_avail(&mut self, desc_index: u16) -> Result<()> {
        unsafe {
            let current_idx = (*self.avail).idx;
            let ring_index = (current_idx % self.queue_size) as usize;
            
            // 🛠️ 修复：传统模式使用原生字节序
            (*self.avail).ring[ring_index] = desc_index;
            
            // 内存屏障
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            
            // 更新索引
            (*self.avail).idx = current_idx.wrapping_add(1);
            
            // 最终屏障
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        }
        Ok(())
    }
    
    /// 检查是否有已完成的请求
    pub fn has_used(&self) -> bool {
    unsafe {
        // 🛠️ 直接读取内存，避免屏障影响诊断
        let current_used_idx = ptr::read_volatile(&(*self.used).idx);
        let last = self.last_used_idx;
        
        if current_used_idx != last {
            return true;
        }
        
        // 🆕 详细检查内存
        if current_used_idx == 0 {
            print("⚠️  USED RING STUCK AT 0 - CHECK DEVICE IMPLEMENTATION\r\n");
        }
        
        false
    }
}

    /// 增强的Used元素获取方法
pub fn get_used_elem(&mut self) -> Option<UsedElem> {
    unsafe {
        let current_used_idx = ptr::read_volatile(&(*self.used).idx);
        
        if current_used_idx == self.last_used_idx {
            print("🔄 No new used elements - current: ");
            print_uint(current_used_idx as u32);
            print(", last: ");
            print_uint(self.last_used_idx as u32);
            print("\r\n");
            return None;
        }
        
        // 🛠️ 修复：正确处理环回
        let used_idx = self.last_used_idx % self.queue_size;
        if used_idx >= self.queue_size {
            print("❌ Invalid used index calculation: ");
            print_uint(used_idx as u32);
            print("\r\n");
            return None;
        }
        
        let elem = ptr::read_volatile(&(*self.used).ring[used_idx as usize]);
        
        // 更新last_used_idx前添加屏障
        core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
        self.last_used_idx = self.last_used_idx.wrapping_add(1);
        
        Some(elem)
    }
}

    /// 释放描述符链
    pub fn free_desc_chain(&mut self, head: u16) {
        let mut current = head;
        let mut count = 0;
        
        // 找到链的末尾
        loop {
            count += 1;
            if let Ok(desc_ptr) = self.get_descriptor_ptr(current) {
                let next = unsafe { (*desc_ptr).next};
                if next == 0 {
                    break;
                }
                current = next;
            } else {
                break;
            }
        }
        
        // 将链重新连接到空闲列表
        if let Ok(desc_ptr) = self.get_descriptor_ptr(current) {
            unsafe {
                (*desc_ptr).next = self.free_head;
            }
        }
        self.free_head = head;
        self.num_free += count;
    }

    /// 获取可用环索引
    pub fn get_avail_idx(&self) -> u16 {
        unsafe {
            // 🆕 添加获取屏障
            core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
            (*self.avail).idx
        }
    }
    
    /// 获取已用环索引
    pub fn get_used_idx(&self) -> u16 {
        unsafe {
            // 🆕 添加获取屏障
            core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
            (*self.used).idx
        }
    }
    
    /// 获取指定索引的描述符
    pub fn get_descriptor(&self, index: u16) -> Option<&Descriptor> {
        if index >= self.queue_size {
            return None;
        }
        
        if let Ok(desc_ptr) = self.get_descriptor_ptr(index) {
            unsafe {
                Some(&*desc_ptr)
            }
        } else {
            None
        }
    }

    /// 🆕 对齐检查方法
    pub fn check_alignment(&self) -> Result<()> {
        let desc_align = core::mem::align_of::<Descriptor>();
        let avail_align = core::mem::align_of::<AvailableRing>();
        let used_align = core::mem::align_of::<UsedRing>();
        
        print("🔍 Alignment check - Desc: ");
        print_uint(desc_align as u32);
        print(", Avail: ");
        print_uint(avail_align as u32);
        print(", Used: ");
        print_uint(used_align as u32);
        print("\r\n");
        
        if desc_align < 16 {
            print("❌ Descriptor alignment insufficient: ");
            print_uint(desc_align as u32);
            print(" < 16\r\n");
            return Err(VirtioError::MemoryNotAligned);
        }
        
        Ok(())
    }

    /// 调试方法：打印队列状态
    pub fn debug_queue_state(&self) {
        print("🔍 VIRTQUEUE STATE:\r\n");
        print("   Queue size: ");
        print_uint(self.queue_size as u32);
        print(", Free head: ");
        print_uint(self.free_head as u32);
        print(", Num free: ");
        print_uint(self.num_free as u32);
        print("\r\n");
        print("   Avail idx: ");
        print_uint(self.get_avail_idx() as u32);
        print(", Used idx: ");
        print_uint(self.get_used_idx() as u32);
        print(", Last used: ");
        print_uint(self.last_used_idx as u32);
        print("\r\n");
        
        // 🆕 打印描述符大小信息
        print("   Descriptor size: ");
        print_uint(self.desc_size as u32);
        print(" bytes\r\n");
    }
    
    /// 调试方法：打印描述符信息
    pub fn debug_descriptor(&self, index: u16) {
        if let Some(desc) = self.get_descriptor(index) {
            print("   [");
            print_uint(index as u32);
            print("] addr=0x");
            print_hex64(desc.addr);
            print(", len=");
            print_uint(desc.len);
            print(", flags=0x");
            print_hex32(desc.flags as u32);
            print(", next=");
            print_uint(desc.next as u32);
            print("\r\n");
        } else {
            print("   [");
            print_uint(index as u32);
            print("] ❌ Invalid descriptor\r\n");
        }
    }
}

/// Virtio描述符标志常量
pub const VIRTQ_DESC_F_NEXT: u16 = 0x1;     // 还有下一个描述符
pub const VIRTQ_DESC_F_WRITE: u16 = 0x2;    // 设备可写入
pub const VIRTQ_DESC_F_INDIRECT: u16 = 0x4; // 间接描述符

impl Default for Descriptor {
    fn default() -> Self {
        Descriptor {
            addr: 0,
            len: 0,
            flags: 0,
            next: 0,
        }
    }
}

impl Default for UsedElem {
    fn default() -> Self {
        UsedElem {
            id: 0,
            len: 0,
        }
    }
}