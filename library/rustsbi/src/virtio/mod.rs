//! Virtio设备驱动模块
// library/rustsbi/src/virtio/mod.rs

use core::ptr;

/// Virtio MMIO 设备寄存器偏移量
pub const VIRTIO_MAGIC_VALUE: usize = 0x000;
pub const VIRTIO_VERSION: usize = 0x004;
pub const VIRTIO_DEVICE_ID: usize = 0x008;
pub const VIRTIO_VENDOR_ID: usize = 0x00C;
pub const VIRTIO_DEVICE_FEATURES: usize = 0x010;
pub const VIRTIO_DRIVER_FEATURES: usize = 0x020;
pub const VIRTIO_STATUS: usize = 0x070;

/// Virtio设备状态位
pub const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
pub const VIRTIO_STATUS_DRIVER: u8 = 2;
pub const VIRTIO_STATUS_FEATURES_OK: u8 = 8;
pub const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
pub const VIRTIO_STATUS_FAILED: u8 = 128;

/// Virtio MMIO 设备
pub struct VirtioMmio {
    base_addr: usize,
}

impl VirtioMmio {
    /// 创建新的Virtio MMIO设备实例
    pub fn new(addr: usize) -> Result<Self> {  // 使用Result而不是VirtioResult
        let mmio = Self { base_addr: addr };
        
        // 验证设备签名
        if !mmio.check_magic() {
            return Err(VirtioError::InvalidMagic);  // 返回VirtioError而不是&str
        }
        
        Ok(mmio)
    }

    /// 验证设备
    pub fn verify_device(&self) -> Result<()> {  // 使用Result
        unsafe {
            let magic = core::ptr::read_volatile((self.base_addr) as *const u32);
            if magic != 0x74726976 { // "virt"
                return Err(VirtioError::InvalidMagic);  // 返回VirtioError
            }
            
            Ok(())
        }
    }

/// 读取32位寄存器
    pub fn read_reg(&self, offset: usize) -> u32 {
        unsafe { ptr::read_volatile((self.base_addr + offset) as *const u32) }
    }
    
    /// 写入32位寄存器
    pub fn write_reg(&mut self, offset: usize, value: u32) {
        unsafe { ptr::write_volatile((self.base_addr + offset) as *mut u32, value) }
    }
    
    /// 检查设备魔数
    pub fn check_magic(&self) -> bool {
        self.read_reg(VIRTIO_MAGIC_VALUE) == 0x74726976 // "virt"
    }
    
    /// 检查设备版本
    pub fn check_version(&self) -> bool {
        self.read_reg(VIRTIO_VERSION) == 2 // Virtio 1.0+
    }
    
    /// 获取设备ID
    pub fn device_id(&self) -> u32 {
        self.read_reg(VIRTIO_DEVICE_ID)
    }
    
    /// 获取设备特性
    pub fn device_features(&self) -> u32 {
        self.read_reg(VIRTIO_DEVICE_FEATURES)
    }
    
    /// 设置驱动特性
    pub fn set_driver_features(&mut self, features: u32) {
        self.write_reg(VIRTIO_DRIVER_FEATURES, features);
    }
    
    /// 设置设备状态
    pub fn set_status(&mut self, status: u8) {
        self.write_reg(VIRTIO_STATUS, status as u32);
    }
}

// 声明子模块
pub mod blk;
pub mod error;
pub mod queue;

// 重新导出子模块的类型
pub use blk::{VirtioBlk, BlkError, BlkDeviceInfo};
pub use error::{VirtioError, Result, VirtioResult};  // 添加VirtioResult
pub use queue::{Virtqueue, Descriptor, AvailableRing, UsedRing};