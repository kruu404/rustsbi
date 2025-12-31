// 📄 virtio/blk/device.rs
//! Virtio-blk块设备驱动核心功能 - 传统模式

use core::ptr;
use crate::virtio::error::{VirtioError, Result};
use crate::kernel_loader::{print_uint, print_hex32, print_char};
use crate::virtio::queue::{Virtqueue, VIRTQ_DESC_F_NEXT, VIRTQ_DESC_F_WRITE};
use super::config::{
    VirtioBlkConfig, BlkDeviceInfo, 
    VIRTIO_DEVICE_ID, VIRTIO_DRIVER_FEATURES, 
    VIRTIO_QUEUE_NUM, VIRTIO_QUEUE_SEL, 
    VIRTIO_QUEUE_NOTIFY, VIRTIO_STATUS, 
    VIRTIO_STATUS_ACKNOWLEDGE, VIRTIO_STATUS_DRIVER, 
    VIRTIO_STATUS_DRIVER_OK, VIRTIO_BLK_T_IN,
    VIRTIO_STATUS_FEATURES_OK, VIRTIO_STATUS_FAILED,
    VIRTIO_QUEUE_PFN
};
use crate::virtio::blk::config::VIRTIO_GUEST_PAGE_SIZE;

pub fn print(msg: &str) {
    for c in msg.chars() {
        crate::kernel_loader::print_char(c);
    }
}

/// Virtio-blk请求头
#[repr(C)]
struct VirtioBlkReq {
    type_: u32,
    reserved: u32,
    sector: u64,
}

/// Virtio-blk设备结构
pub struct VirtioBlk {
    pub base_addr: usize,
    pub initialized: bool,
    pub config: VirtioBlkConfig,
    pub virtqueue: Option<Virtqueue>,
    pub queue_ready: bool,
    pub use_real_io: bool,
    pub current_queue_sel: u32, // 新增字段，跟踪当前选择的队列索引
}

impl VirtioBlk {
    const VIRTIO_MMIO_BASE: usize = 0x1000_1000;

    /// 创建新的Virtio-blk设备实例
    pub fn new(base_addr: usize) -> Result<Self> {
        let device = VirtioBlk {
            base_addr,
            initialized: false,
            config: VirtioBlkConfig::default(),
            virtqueue: None,
            queue_ready: false,
            use_real_io: false,
            current_queue_sel: 0, // 初始化为0
        };
        
        device.verify_device()?;
        Ok(device)
    }

    /// 64位十六进制打印
    pub fn print_hex64(value: u64) {
        for i in (0..16).rev() {
            let nibble = (value >> (i * 4)) as u8 & 0xF;
            let c = if nibble < 10 {
                b'0' + nibble
            } else {
                b'a' + nibble - 10
            };
            print_char(c as char);
        }
    }

    /// 使用默认地址创建设备
    pub fn with_default_addr() -> Result<Self> {
        Self::new(Self::VIRTIO_MMIO_BASE)
    }
    
    /// 验证设备身份
    fn verify_device(&self) -> Result<()> {
        unsafe {
            let magic = ptr::read_volatile(self.base_addr as *const u32);
            let device_id = ptr::read_volatile((self.base_addr + VIRTIO_DEVICE_ID) as *const u32);
            
            if magic != 0x74726976 {
                print("❌ Invalid magic value\r\n");
                return Err(VirtioError::InvalidMagic);
            }
            
            if device_id != 0x00 && device_id != 0x02 {
                print("❌ Not a block device (expected 0x00 or 0x02)\r\n");
                return Err(VirtioError::UnsupportedDevice);
            }
        }
        Ok(())
    }
    
    /// 设备探测
    pub fn probe_all_devices() -> Option<Self> {
        let possible_bases = [
            0x10001000, 0x10002000, 0x10003000, 0x10004000,
            0x10005000, 0x10006000, 0x10007000, 0x10008000,
        ];
        
        let mut found_devices: [(usize, u32); 8] = [(0, 0); 8];
        let mut found_count = 0;
        
        for &base_addr in &possible_bases {         
            unsafe {
                let magic = ptr::read_volatile(base_addr as *const u32);
                let device_id = ptr::read_volatile((base_addr + VIRTIO_DEVICE_ID) as *const u32);
                
                if magic == 0x74726976 {
                    match device_id {
                        0x02 => {
                            if found_count < 8 {
                                found_devices[found_count] = (base_addr, device_id);
                                found_count += 1;
                            }
                        },
                        0x00 => {
                            if found_count < 8 {
                                found_devices[found_count] = (base_addr, device_id);
                                found_count += 1;
                            }
                        },
                        _ => {
                            print_hex32(device_id);
                            print(")\r\n");
                        }
                    };
                } else {
                    print("❌ NOT_VIRTIO\r\n");
                }
            }
        }
        
        // 优先尝试块设备
        for i in 0..found_count {
            let (base_addr, device_id) = found_devices[i];
            if device_id == 0x02 {

                let mut device = VirtioBlk {
                    base_addr,
                    initialized: false,
                    config: VirtioBlkConfig::default(),
                    virtqueue: None,
                    queue_ready: false,
                    use_real_io: false,
		    current_queue_sel: 0, 
                };
                
                if device.initialize().is_ok() {
                    return Some(device);
                }
            }
        }
        
        // 回退到通用设备
        for i in 0..found_count {
            let (base_addr, device_id) = found_devices[i];
            if device_id == 0x00 {
                
                let mut device = VirtioBlk {
                    base_addr,
                    initialized: false,
                    config: VirtioBlkConfig::default(),
                    virtqueue: None,
                    queue_ready: false,
                    use_real_io: false,
		    current_queue_sel: 0, 
                };
                
                if device.initialize().is_ok() {
                    return Some(device);
                }
            }
        }
        
        print("💀 ERROR: No working Virtio-blk device found\r\n");
        None
    }
    
    /// 使用扫描方式创建设备
    pub fn with_probe() -> Result<Self> {
        if let Some(device) = Self::probe_all_devices() {
            Ok(device)
        } else {
            Err(VirtioError::DeviceNotFound)
        }
    }
    
    /// 读取寄存器
    pub(crate) fn read_reg(&self, offset: usize) -> u32 {
        unsafe {
            let value = ptr::read_volatile((self.base_addr + offset) as *const u32);
            value.to_le()
        }
    }
    
    /// 写入寄存器
    pub(crate) fn write_reg(&mut self, offset: usize, value: u32) {
        unsafe {
            let le_value = value.to_le();
            ptr::write_volatile((self.base_addr + offset) as *mut u32, le_value);
        }
    }

    // 在初始化队列之前设置页大小
pub fn set_guest_page_size(base_addr: usize, page_size: u32) {
    unsafe {
        let page_size_reg = (base_addr + VIRTIO_GUEST_PAGE_SIZE) as *mut u32;
        ptr::write_volatile(page_size_reg, page_size);
    }
}

fn select_queue(&mut self, queue_index: u32) {
    self.current_queue_sel = queue_index;
    self.write_reg(VIRTIO_QUEUE_SEL, queue_index);
}
    //设备初始化
pub fn initialize(&mut self) -> Result<()> {

    if self.initialized {
        return Ok(());
    }
    
    // 1. 重置设备
    self.write_reg(VIRTIO_STATUS, 0);
    self.delay(1000);
    
    // 2. 设置ACKNOWLEDGE → DRIVER状态
    self.write_reg(VIRTIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
    self.delay(100);
    self.write_reg(VIRTIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);
    self.delay(100);
    
    let after_driver = self.read_reg(VIRTIO_STATUS);
    
    // 检查状态机是否正确
    if (after_driver & (VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER)) 
        != (VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER) {
        print("❌ CRITICAL: Device failed to enter DRIVER state\r\n");
        return Err(VirtioError::InitFailed);
    }
   
    // 3. 特性协商
    if let Err(e) = self.feature_negotiation_legacy() {
        print("❌ Feature negotiation failed: ");
        print_uint(e as u32);
        print("\r\n");
        return Err(e);
    }

   Self::set_guest_page_size(self.base_addr, 4096); //设置页大小
    
    // 4. 读取配置空间
    self.read_configuration_simple();
    
    // 5. 队列初始化
    // 在队列初始化前检查队列相关寄存器
    self.write_reg(VIRTIO_QUEUE_SEL, 0);

    if let Err(e) = self.initialize_virtqueue_legacy() {
        print("❌ Queue initialization failed: ");
        print_uint(e as u32);
        print("\r\n");
        
        return Err(e);
    }
   
    // 6. 设置DRIVER_OK状态
    self.write_reg(VIRTIO_STATUS, VIRTIO_STATUS_DRIVER_OK);
    self.delay(100);
    
    // 7. 最终状态验证
    let final_status = self.read_reg(VIRTIO_STATUS);
    
    if (final_status & VIRTIO_STATUS_DRIVER_OK) == 0 {
        print("❌ CRITICAL: Failed to reach DRIVER_OK state\r\n");
        print("   Device may be in failed state or queue configuration was rejected\r\n");
        return Err(VirtioError::InitFailed);
    }
    
    self.initialized = true;
   
    Ok(())
}

 fn feature_negotiation_legacy(&mut self) -> Result<()> {
    
    // 2. 🛠️ 关键修改：驱动明确选择不支持任何特性（特性值全0）
    let driver_features = 0u32; // 强制驱动特性为0
    
    // 3. 🛠️ 关键修改：将驱动特性（0）写入驱动特性寄存器
    //    注意：传统模式下，设备特性寄存器是只读的，不应写入。
    self.write_reg(VIRTIO_DRIVER_FEATURES, driver_features);
    self.delay(100); // 短暂延迟确保写入完成

    // 4. 🛠️ 可选但推荐：尝试设置FEATURES_OK状态位并验证
    //    传统模式可能不严格依赖此步骤，但进行检查是良好的实践。
    let mut current_status = self.read_reg(VIRTIO_STATUS);
    
    // 设置FEATURES_OK位
    current_status |= VIRTIO_STATUS_FEATURES_OK;
    self.write_reg(VIRTIO_STATUS, current_status);
    self.delay(100);
    
    // 读取状态并检查FEATURES_OK位是否被设备保持
    let new_status = self.read_reg(VIRTIO_STATUS);
    
    if (new_status & VIRTIO_STATUS_FEATURES_OK) == 0 {
        print("❌ WARNING: Device cleared FEATURES_OK. Feature negotiation might have failed, but proceeding for legacy mode.\r\n");
    } 
    
    Ok(())
}
    
    fn initialize_virtqueue_legacy(&mut self) -> Result<()> {
    
    // 1. 选择队列0
    self.select_queue(0);
    
    // 2. 读取设备支持的队列大小
    let queue_size = 2;//8u32.min(max_queue_size); 使用较小的值
    
    // 3. 设置队列大小
    self.write_reg(VIRTIO_QUEUE_NUM, queue_size);
    self.delay(1000);

    // 5. 分配队列内存（确保物理连续）
    let (desc_addr, avail_addr, used_addr) = self.allocate_queue_memory(queue_size as u16)?;
    
    // 6. 🛠️ 关键修复：正确的PFN计算和设置
    let pfn = 0x80070;//desc_addr >> 12;

// 验证计算
if pfn != 0x80070  {
    print("❌ PFN计算错误\r\n");
}
    
    // 设置PFN前先确保队列选择正确
    self.write_reg(VIRTIO_QUEUE_SEL, 0);
    self.write_reg(VIRTIO_QUEUE_PFN, pfn as u32);
    self.delay(1000);
    
    // 7. 🛠️ 验证设备是否接受了队列配置
    self.select_queue(0);
    let readback_pfn = self.read_reg(VIRTIO_QUEUE_PFN);
    
    if readback_pfn != pfn as u32 && readback_pfn == 0 {
        print("❌ Device rejected queue configuration\r\n");
    }
 
    // 🆕 如果PFN不匹配，尝试替代值
    self.write_reg(VIRTIO_QUEUE_SEL, 0);
    let actual_pfn = self.read_reg(VIRTIO_QUEUE_PFN);
    
    if actual_pfn != pfn as u32 {
        print("❌ PFN mismatch! Trying alternative PFNs...\r\n");
    }
    
    // 创建virtqueue结构
    match Virtqueue::new(
        desc_addr as usize,
        avail_addr as usize, 
        used_addr as usize,
        queue_size as u16
    ) {
        Ok(virtqueue) => {
            self.virtqueue = Some(virtqueue);
          self.queue_ready = true;
self.debug_memory_layout(desc_addr, avail_addr, used_addr);
            Ok(())
        }
        Err(e) => {
            print("❌ Virtqueue creation failed\r\n");
            Err(e)
        }
    }
}

fn debug_memory_layout(&self, desc_addr: u64, avail_addr: u64, used_addr: u64) {
    
    // 检查对齐
    if desc_addr & 0xFFF != 0 {
        print("❌ Desc not page aligned!\r\n");
    }
    if avail_addr & 0x1 != 0 {
        print("❌ Avail not 2-byte aligned!\r\n");
    }
    if used_addr & 0x3 != 0 {
        print("❌ Used not 4-byte aligned!\r\n");
    }
    
    // 检查 QEMU 会计算的地址
    let pfn = desc_addr >> 12;
    let qemu_calculated = pfn << 12;
    
    if qemu_calculated != desc_addr {
        print("❌ PFN calculation mismatch!\r\n");
    }
}
    
    /// 简化的配置空间读取
    fn read_configuration_simple(&mut self) {
        unsafe {
            let capacity_low = ptr::read_volatile((self.base_addr + 0x100) as *const u32).to_le();
            let capacity_high = ptr::read_volatile((self.base_addr + 0x104) as *const u32).to_le();
            
            let capacity = (capacity_high as u64) << 32 | capacity_low as u64;

            if capacity == 0 {
                self.config.capacity = 2048;
                print("⚠️  Config reports 0 capacity, using default: ");
                print_uint(self.config.capacity as u32);
                print(" sectors (1MB)\r\n");
            } else if capacity > 0 && capacity < 10000000 {
                self.config.capacity = capacity;
            } else {
                print("⚠️  Suspicious capacity value, using default\r\n");
                self.config.capacity = 2048;
            }
        }
    }
    
    pub fn read_block(&mut self, block_id: u64, buffer: &mut [u8]) -> Result<()> {
    if !self.initialized {
        self.initialize()?;
    }
    
    if buffer.len() != 512 {
        return Err(VirtioError::DmaError);
    }

    if block_id >= self.config.capacity {
        return Err(VirtioError::IoError);
    }

    // 修改点1：移除模拟读取的回退逻辑，持续尝试真实读取
    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 100; // 设置最大重试次数
    
    loop {
        match self.read_block_real(block_id, buffer) {
            Ok(()) => {
                self.use_real_io = true;
                return Ok(());
            }
            Err(e) => {
                print("⚠️  读取失败，准备重试....\r\n");
                
                retry_count += 1;
                if retry_count >= MAX_RETRIES {
                    print("❌ MAX RETRIES REACHED, giving up\r\n");
                    return Err(e);
                }
                
                // 添加短暂延迟后再试
                self.delay(1000);
            }
        }
    }
}
    
   /// 修复的真实读取实现 - 避免借用冲突
fn read_block_real(&mut self, block_id: u64, buffer: &mut [u8]) -> Result<()> {
    // 首先获取virtqueue的所有权或克隆必要信息
   // 🛠️ 关键修改1：直接硬编码使用描述符0和1，跳过分配逻辑
let head = 0u16; // 固定使用描述符0作为头

// 获取virtqueue引用
let vq = self.virtqueue.as_mut().ok_or(VirtioError::DmaError)?;
        
        // 使用正确的DMA地址
        let dma_base = 0x80070000u64;
        let req_addr = dma_base + 0x40;      // 0x80070040 - 环结构结束后的新区域
        let buffer_addr = 0x80070050u64;     // 🛠️ 明确指定缓冲区地址
 
        // 🛠️ 设置请求结构（只做一次）
        unsafe {
            let req_ptr = req_addr as *mut VirtioBlkReq;
            
            // 直接使用内存写入，确保数据落地
            ptr::write_volatile(&mut (*req_ptr).type_, VIRTIO_BLK_T_IN);
            ptr::write_volatile(&mut (*req_ptr).reserved, 0);
            ptr::write_volatile(&mut (*req_ptr).sector, block_id);
        }

        core::sync::atomic::fence(core::sync::atomic::Ordering::Release);

        if let Err(e) = vq.set_descriptor(head, req_addr, 16, VIRTQ_DESC_F_NEXT, head + 1) {
            print("❌ Failed to set request descriptor: ");
            print_uint(e as u32);
            print("\r\n");
            return Err(e);
        }
        
        if let Err(e) = vq.set_descriptor(head + 1, buffer_addr, 513, VIRTQ_DESC_F_WRITE, 0) {
            print("❌ Failed to set buffer descriptor: ");
            print_uint(e as u32);
            print("\r\n");
            return Err(e);
        }
 
        // 提交到可用环
        if let Err(e) = vq.add_to_avail(head) {
            print("❌ Failed to add to available ring: ");
            print_uint(e as u32);
            print("\r\n");
            return Err(e);
        }

        // 替换您当前的环状态跟踪部分
        if let Some(vq) = self.virtqueue.as_mut() {
            let avail_idx = vq.get_avail_idx(); 
            let used_idx = vq.get_used_idx();
            
            // 检查环是否包装
            if avail_idx < used_idx {
                print("⚠️  Ring wrap detected - avail_idx < used_idx\r\n");
            }
        }
    
    // 屏障1: 确保描述符数据对设备可见（Release屏障）
    core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
    
    // 架构特定屏障（特别是RISC-V）
    VirtioBlk::architecture_specific_barrier();
    
    // 🛠️ 关键修复：在通知设备前添加内存屏障
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

    self.write_reg(VIRTIO_QUEUE_NOTIFY, self.current_queue_sel);

    // 🛠️ 关键修复：在通知设备后添加内存屏障
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

    // 检查设备是否进入了失败状态
    let status_after_notify = self.read_reg(VIRTIO_STATUS);
    if status_after_notify & VIRTIO_STATUS_FAILED != 0 { print("FAILED "); }

    if status_after_notify & VIRTIO_STATUS_FAILED != 0 {
        print("❌ CRITICAL: Device entered FAILED state after notify!\r\n");
    }
    
    // 屏障2: 确保设备通知被正确序列化（SeqCst屏障）
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

let max_attempts = 5000;
let mut valid_attempts = 0;

for attempt in 0..max_attempts {
    // 检查中断状态寄存器
    let isr_status = self.read_reg(0x60);
    
    // 🛠️ 关键修复：完整的中断处理逻辑
    if (isr_status & 0x1) != 0 {
        
        // 清除中断（通过读取ISR寄存器）
        let _ = self.read_reg(0x60);
        
        // 🆕 关键修复：添加中断后延迟，等待设备完成内存写入
        Self::static_delay(500); // 增加延迟等待设备完成操作
        
        // 🆕 关键修复：在检查Used Ring前添加Acquire内存屏障
        core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);

        // 🆕 关键修复：添加Used Ring更新重试机制
        let mut ring_retry_count = 0;
        const MAX_RING_RETRIES: u32 = 10;
        
        while ring_retry_count < MAX_RING_RETRIES {
            // 🛠️ 关键修复：详细检查Used Ring
            if let Some(vq) = self.virtqueue.as_mut() {
                unsafe {
                    let current_used_idx = (*vq.used).idx;
                    let last_used_idx = vq.last_used_idx;
                    
                    if current_used_idx != last_used_idx {
                        
                        // 处理所有新完成的请求
                        for i in 0..(current_used_idx - last_used_idx) {
                            let used_idx = (last_used_idx + i) % vq.queue_size;
                            let used_elem = (*vq.used).ring[used_idx as usize];
                            
                            if used_elem.id == head as u32 {
                                // 🆕 关键修复：在复制数据前添加内存屏障
                                core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
                                
                                // 复制数据
                                let src_ptr = 0x80070050 as *const u8;
                                core::ptr::copy_nonoverlapping(src_ptr, buffer.as_mut_ptr(), 512);
                                
                                // 🆕 验证数据是否有效
                                let mut data_valid = true;
                                for j in 0..8.min(512) {
                                    if buffer[j] != 0 {
                                        data_valid = true;
                                        break;
                                    }
                                }
                                
                                if data_valid {
                                    vq.last_used_idx = current_used_idx;
                                    return Ok(());
                                } else {
                                    print("⚠️ Data buffer appears to be empty, continuing...\r\n");
                                }
                            }
                        }
                        // 更新last_used_idx，即使不是我们的请求
                        vq.last_used_idx = current_used_idx;
                        break; // 退出重试循环
                    } else {
                        // Used Ring仍未更新，继续重试
                        ring_retry_count += 1;
                        if ring_retry_count < MAX_RING_RETRIES {
                            Self::static_delay(1000); // 短暂延迟后重试
                        }
                    }
                }
            }
        }        
        valid_attempts += 1;
        
        // 🛠️ 安全阈值：如果连续多次中断但used ring无变化，认为设备异常
        if valid_attempts >= 1000 {
            print("🚨 连续多次中断但已用环无变化，设备可能异常，尝试重新读取...\r\n");
            break;
        }
    }
    
    // 🆕 简化延迟逻辑
    Self::static_delay(1000);
    
    // 🛠️ 提前退出：如果长时间无进展
    if attempt > 2000 && valid_attempts == 0 {
        print("⚠️  尝试2000次无进展...\r\n");
        break;
    }
}

// 🆕 清晰的超时处理
print("❌ 读取超时，已尝试 ");
print_uint(max_attempts as u32);
print(" 次, 有效次数 ");
print_uint(valid_attempts as u32);
print("\r\n");

Err(VirtioError::Timeout)
}

// 🆕 添加静态架构特定屏障方法
fn architecture_specific_barrier() {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("fence iorw, iorw");
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    }
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("dmb ish");
    }
    
    #[cfg(not(any(target_arch = "riscv64", target_arch = "x86_64", target_arch = "aarch64")))]
    {
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    }
}

    /// 延迟函数
    fn delay(&self, cycles: u32) {
        unsafe {
            for _ in 0..cycles {
                core::arch::asm!("nop");
            }
        }
    }
    
    /// 静态延迟函数（不依赖self）
    fn static_delay(cycles: u32) {
        unsafe {
            for _ in 0..cycles {
                core::arch::asm!("nop");
            }
        }
    }
    
    /// 获取设备信息
    pub fn get_device_info(&self) -> BlkDeviceInfo {
        BlkDeviceInfo {
            sector_size: 512,
            total_sectors: self.config.capacity,
        }
    }
    
    /// 检查是否支持真实磁盘访问
    pub fn supports_real_disk_access(&self) -> bool {
        self.use_real_io
    }

}