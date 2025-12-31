// 📄 virtio/blk/mod.rs
//! Virtio-blk块设备驱动模块
//! 此文件导出所有相关模块

pub mod device;
pub mod config;
pub mod memory;

// 从父模块导入错误类型（正确路径）
pub use crate::virtio::error::{VirtioError as BlkError, Result as BlkResult};
pub use device::VirtioBlk;
pub use config::{BlkDeviceInfo, VirtioBlkConfig};

/// 错误转换函数
pub fn from_virtio_error(err: BlkError) -> BlkError {
    err
}

// 可选：为了保持向后兼容性，可以添加这些别名
pub use BlkError as VirtioError;
pub use BlkResult as Result;