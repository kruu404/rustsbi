// 📄 virtio/blk/config.rs
//! 配置空间、寄存器偏移量、常量定义 - 传统模式 (Legacy Mode)
//! 关键修改：寄存器偏移量遵循 Virtio 传统模式规范，与现代模式不同。

use core::ptr;
use crate::kernel_loader::print_hex32;
use crate::virtio::error::Result;
use crate::virtio::blk::device::print;
use crate::kernel::print_hex64;

// ========== ✅ 修正后的传统模式寄存器偏移量 (Legacy Mode Offsets) ==========
// 核心区别：传统模式使用一套独立的、更简单的寄存器映射，通常从 0x00 开始连续分布。
// 参考：Virtio 规范中传统设备（Transitional Devices）的 MMIO 布局。

pub const VIRTIO_MAGIC: usize = 0x000;         // 魔数，用于识别Virtio设备
pub const VIRTIO_VERSION: usize = 0x004;       // 版本号，传统设备应为1
pub const VIRTIO_DEVICE_ID: usize = 0x008;     // 设备ID (0x02 代表块设备)
pub const VIRTIO_VENDOR_ID: usize = 0x00C;     // 厂商ID (0x554D4551 代表QEMU)

// 🛠️ 核心修正：传统模式特性寄存器偏移量
pub const VIRTIO_DEVICE_FEATURES: usize = 0x010;  // 设备支持的特性（32位）
pub const VIRTIO_DRIVER_FEATURES: usize = 0x020;  // 驱动接受的特性（32位）

// 🛠️ 核心修正：传统模式队列寄存器偏移量
pub const VIRTIO_QUEUE_SEL: usize = 0x030;       // 选择要操作的队列索引
pub const VIRTIO_QUEUE_NUM_MAX: usize = 0x034;  // 所选队列的最大大小
pub const VIRTIO_QUEUE_NUM: usize = 0x038;      // 设置所选队列的大小
pub const VIRTIO_QUEUE_ALIGN: usize = 0x03C;   // 队列内存对齐要求（传统模式可能忽略）
pub const VIRTIO_QUEUE_PFN: usize = 0x040;      // 🎯 关键：队列的物理页帧号 (PFN)
pub const VIRTIO_GUEST_PAGE_SIZE: usize = 0x028;  // 🎯 页大小寄存器

pub const VIRTIO_QUEUE_NOTIFY: usize = 0x050;   // 队列通知寄存器，写入队列索引以通知设备
pub const VIRTIO_STATUS: usize = 0x070;         // 设备状态寄存器

// ========== 设备状态位定义 (Device Status Bits) ==========
// 这些状态位的含义在传统模式和现代模式中是相同的。
pub const VIRTIO_STATUS_ACKNOWLEDGE: u32 = 1;      // 操作系统已发现设备
pub const VIRTIO_STATUS_DRIVER: u32 = 2;           // 操作系统驱动已加载
pub const VIRTIO_STATUS_DRIVER_OK: u32 = 4;        // 驱动就绪，设备可正常操作
pub const VIRTIO_STATUS_FEATURES_OK: u32 = 8;      // 特性协商完成（传统模式下可能不严格检查）
pub const VIRTIO_STATUS_FAILED: u32 = 0x80;        // 设备发生错误

// ========== 块设备相关常量 (Block Device Constants) ==========
// 请求类型
pub const VIRTIO_BLK_T_IN: u32 = 0;    // 读取请求
pub const VIRTIO_BLK_T_OUT: u32 = 1;   // 写入请求
pub const VIRTIO_BLK_T_FLUSH: u32 = 4; // 刷新缓存请求

// 请求状态
pub const VIRTIO_BLK_S_OK: u8 = 0;     // 操作成功
pub const VIRTIO_BLK_S_IOERR: u8 = 1;  // 设备I/O错误
pub const VIRTIO_BLK_S_UNSUPP: u8 = 2; // 请求类型不支持

// ========== 可选特性位 (Optional Feature Bits) ==========
// 驱动和设备通过特性位协商高级功能。您的项目若只使用基本读写，可忽略大多数。
pub const VIRTIO_BLK_F_SIZE_MAX: u32 = 1 << 1;   // 最大段大小
pub const VIRTIO_BLK_F_SEG_MAX: u32 = 1 << 2;    // 最大段数
pub const VIRTIO_BLK_F_GEOMETRY: u32 = 1 << 4;   // 磁盘几何信息
pub const VIRTIO_BLK_F_RO: u32 = 1 << 5;         // 只读设备
pub const VIRTIO_BLK_F_BLK_SIZE: u32 = 1 << 6;   // 块大小（扇区大小）
pub const VIRTIO_BLK_F_FLUSH: u32 = 1 << 9;      // 缓存刷新命令
pub const VIRTIO_BLK_F_TOPOLOGY: u32 = 1 << 10;  // 拓扑信息
pub const VIRTIO_BLK_F_CONFIG_WCE: u32 = 1 << 11; // 可配置写回缓存
pub const VIRTIO_F_VERSION_1: u32 = 1 << 31;     // 标志现代模式（传统模式不协商此位）

/// 设备配置空间
/// 位于MMIO基地址偏移 0x100 处，用于获取磁盘容量等信息。
#[repr(C)]
#[derive(Default, Debug)]
pub struct VirtioBlkConfig {
    pub capacity: u64, // 磁盘总容量，以512字节扇区为单位
    // 根据协商的特性，后边可能还有其他字段，但基本读取只需关注 capacity
}

/// 块设备信息结构
#[derive(Debug, Clone)]
pub struct BlkDeviceInfo {
    pub sector_size: u32,    // 扇区大小（字节），通常为512
    pub total_sectors: u64,  // 总扇区数
}

/// 🆕 传统模式特定功能
impl VirtioBlkConfig {
    /// 从MMIO设备读取配置空间
    pub fn read_legacy_config(base_addr: usize) -> Result<Self> {
        let mut config = VirtioBlkConfig::default();
        
        unsafe {
            // 读取容量（低32位和高32位）
            let capacity_low = ptr::read_volatile((base_addr + 0x100) as *const u32);
            let capacity_high = ptr::read_volatile((base_addr + 0x104) as *const u32);
            
            // 🛠️ 核心修正：将 `#[cfg]` 属性应用于整个赋值语句块
            // 传统模式下，配置空间字段使用主机原生字节序
            config.capacity = {
                // 这个属性现在应用于代码块，是稳定版支持的标准用法
                #[cfg(target_endian = "little")]
                {
                    // 小端字节序：低字节在前
                    ((capacity_high as u64) << 32) | (capacity_low as u64)
                }
                // 这个属性也应用于代码块
                #[cfg(target_endian = "big")]
                {
                    // 大端字节序：高字节在前
                    ((capacity_low as u64) << 32) | (capacity_high as u64)
                }
                // 如果未指定字节序，提供一个默认值（通常不会发生）
                #[cfg(not(any(target_endian = "little", target_endian = "big")))]
                {
                    compile_error!("Unsupported endianness");
                    0 // 不会执行，仅为语法完整
                }
            };
            
            // 可选：打印调试信息
            print("📊 Legacy config capacity: low=0x");
            print_hex32(capacity_low);
            print(", high=0x");
            print_hex32(capacity_high);
            print(" -> total_sectors=0x");
            // 这里需要您的 print_hex64 函数
            print_hex64(config.capacity);
            print("\r\n");
        }
        
        Ok(config)
    }
}

/// 🆕 检测设备模式
pub fn is_legacy_mode(base_addr: usize) -> bool {
    unsafe {
        let version = ptr::read_volatile((base_addr + VIRTIO_VERSION) as *const u32);
        // 传统设备版本号为1，现代设备版本号为2[3,5](@ref)
        version == 1
    }
}