// library/rustsbi/src/virtio/error.rs
#![allow(dead_code)]

use core::fmt;

/// Virtio设备错误类型（基于Virtio 1.1规范）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioError {
    // 设备发现和验证错误
    DeviceNotFound,
    InvalidMagic,
    UnsupportedVersion,
    UnsupportedDevice,
    DeviceError,
    
    // 设备初始化错误
    InitFailed,
    FeaturesNegotiationFailed,
    QueueSetupFailed,
    ConfigAccessFailed,
    
    // DMA和传输错误
    DmaError,
    IoError,
    BufferTooSmall,
    InvalidParam,
    
    // 队列操作错误
    QueueFull,
    QueueEmpty,
    DescriptorChainTooLong,
    InvalidDescriptor,
    
    // 超时和状态错误
    Timeout,
    NotReady,
    AlreadyInitialized,
    
    // 设备特定错误
    FileSystemError,
    CryptoError,
    NetworkError,
    BlockError,
    
    // 内存错误
    OutOfMemory,
    MemoryNotAligned,
    
    // 通用错误
    InternalError,
    UnsupportedOperation,
}

impl fmt::Display for VirtioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl VirtioError {
    /// 获取错误描述信息
    pub fn as_str(&self) -> &'static str {
        match self {
            // 设备发现和验证错误
            Self::DeviceNotFound => "Virtio device not found",
            Self::InvalidMagic => "Invalid Virtio magic value",
            Self::UnsupportedVersion => "Unsupported Virtio version",
            Self::UnsupportedDevice => "Unsupported device type",
	    Self::DeviceError => "device error",
            
            // 设备初始化错误
            Self::InitFailed => "Device initialization failed",
            Self::FeaturesNegotiationFailed => "Features negotiation failed",
            Self::QueueSetupFailed => "Virtqueue setup failed",
            Self::ConfigAccessFailed => "Device configuration access failed",
            
            // DMA和传输错误
            Self::DmaError => "DMA transfer error",
            Self::IoError => "I/O operation error",
            Self::BufferTooSmall => "Buffer too small for operation",
            Self::InvalidParam => "Invalid parameter",
            
            // 队列操作错误
            Self::QueueFull => "Virtqueue is full",
            Self::QueueEmpty => "virtqueue is empty",
            Self::DescriptorChainTooLong => "Descriptor chain too long",
            Self::InvalidDescriptor => "Invalid descriptor",
            
            // 超时和状态错误
            Self::Timeout => "Operation timeout",
            Self::NotReady => "Device not ready",
            Self::AlreadyInitialized => "Device already initialized",
            
            // 设备特定错误
            Self::FileSystemError => "Filesystem error",
            Self::CryptoError => "Cryptographic operation error",
            Self::NetworkError => "Network operation error",
            Self::BlockError => "Block device operation error",
            
            // 内存错误
            Self::OutOfMemory => "Out of memory",
            Self::MemoryNotAligned => "Memory not properly aligned",
            
            // 通用错误
            Self::InternalError => "Internal virtio error",
            Self::UnsupportedOperation => "Unsupported operation",
        }
    }
    
    /// 检查错误是否可恢复
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::QueueFull
            | Self::Timeout
            | Self::NotReady
            | Self::BufferTooSmall => true,
            _ => false,
        }
    }
    
    /// 检查错误是否需要设备重置
    pub fn requires_reset(&self) -> bool {
        match self {
            Self::InitFailed
            | Self::FeaturesNegotiationFailed
            | Self::QueueSetupFailed
            | Self::DmaError
            | Self::InternalError => true,
            _ => false,
        }
    }
}

/// Virtio操作结果类型别名
pub type Result<T> = core::result::Result<T, VirtioError>;

// 为no_std环境实现错误trait
#[cfg(feature = "nightly")]
impl core::error::Error for VirtioError {}

/// 设备状态常量（基于Virtio 1.1规范）
pub mod status {
    /// 设备状态位定义[2](@ref)
    pub const VIRTIO_STATUS_ACKNOWLEDGE: u32 = 1;
    pub const VIRTIO_STATUS_DRIVER: u32 = 2;
    pub const VIRTIO_STATUS_DRIVER_OK: u32 = 4;
    pub const VIRTIO_STATUS_FEATURES_OK: u32 = 8;
    pub const VIRTIO_STATUS_DEVICE_NEEDS_RESET: u32 = 64;
    pub const VIRTIO_STATUS_FAILED: u32 = 128;
}

/// 特性位常量（部分常用特性）
pub mod features {
    /// 间接描述符特性[2](@ref)
    pub const VIRTIO_F_RING_INDIRECT_DESC: u64 = 1 << 28;
    /// 事件索引特性[2](@ref)
    pub const VIRTIO_F_RING_EVENT_IDX: u64 = 1 << 29;
    /// Virtio 1.0+ 规范兼容性[2](@ref)
    pub const VIRTIO_F_VERSION_1: u64 = 1 << 32;
    /// 平台访问特性[2](@ref)
    pub const VIRTIO_F_ACCESS_PLATFORM: u64 = 1 << 33;
    /// 打包虚拟队列特性[2](@ref)
    pub const VIRTIO_F_RING_PACKED: u64 = 1 << 34;
}

/// 队列相关常量
pub mod queue {
    /// 描述符标志[2](@ref)
    pub const VIRTQ_DESC_F_NEXT: u16 = 0x1;      // 还有下一个描述符
    pub const VIRTQ_DESC_F_WRITE: u16 = 0x2;    // 设备可写入
    pub const VIRTQ_DESC_F_INDIRECT: u16 = 0x4;  // 间接描述符
    
    /// 最大队列大小
    pub const VIRTQUEUE_MAX_SIZE: u16 = 32768;
}

/// 块设备特定错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlkError {
    /// 底层Virtio错误
    VirtioError(VirtioError),
    /// 设备只读
    ReadOnly,
    /// 无效的扇区访问
    InvalidSector,
    /// 超出设备容量
    CapacityExceeded,
    /// 不支持的操作
    UnsupportedOperation,
    /// 媒体变化
    MediaChanged,
}

impl fmt::Display for BlkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VirtioError(e) => write!(f, "Block device error: {}", e),
            Self::ReadOnly => write!(f, "Block device is read-only"),
            Self::InvalidSector => write!(f, "Invalid sector access"),
            Self::CapacityExceeded => write!(f, "Capacity exceeded"),
            Self::UnsupportedOperation => write!(f, "Unsupported block operation"),
            Self::MediaChanged => write!(f, "Media changed"),
        }
    }
}

impl BlkError {
    /// 获取错误描述信息
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VirtioError(_) => "Underlying virtio error",
            Self::ReadOnly => "Block device is read-only",

            Self::InvalidSector => "Invalid sector access",
            Self::CapacityExceeded => "Capacity exceeded",
            Self::UnsupportedOperation => "Unsupported block operation",
            Self::MediaChanged => "Media changed",
        }
    }
}

/// 网络设备特定错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetError {
    /// 底层Virtio错误
    VirtioError(VirtioError),
    /// 无效的数据包
    InvalidPacket,
    /// 缓冲区不足
    InsufficientBuffer,
    /// 链接断开
    LinkDown,
    /// MAC地址错误
    InvalidMacAddress,
}

impl From<VirtioError> for NetError {
    fn from(err: VirtioError) -> Self {
        NetError::VirtioError(err)
    }
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VirtioError(e) => write!(f, "Network device error: {}", e),
            Self::InvalidPacket => write!(f, "Invalid packet"),
            Self::InsufficientBuffer => write!(f, "Insufficient buffer space"),
            Self::LinkDown => write!(f, "Network link down"),
            Self::InvalidMacAddress => write!(f, "Invalid MAC address"),
        }
    }
}

/*
impl From<VirtioError> for core::convert::Infallible {
    fn from(err: VirtioError) -> Self {
        // 对于Infallible，实际上不应该有转换，因为VirtioError是可失败的
        // 这里使用unreachable!()表示这个转换不应该发生
        unreachable!("VirtioError should not be converted to Infallible: {}", err)
    }
}
*/

/// VirtioResult类型别名（与Result相同，用于兼容性）
pub type VirtioResult<T> = core::result::Result<T, VirtioError>;

// 为BlkError添加From<VirtioError>实现（修复冲突）
impl From<VirtioError> for BlkError {
    fn from(err: VirtioError) -> Self {
        BlkError::VirtioError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(VirtioError::DeviceNotFound.as_str(), "Virtio device not found");
        assert_eq!(VirtioError::QueueFull.as_str(), "Virtqueue is full");
    }

    #[test]
    fn test_error_recoverable() {
        assert!(VirtioError::QueueFull.is_recoverable());
        assert!(!VirtioError::InitFailed.is_recoverable());
    }

    #[test]
    fn test_error_requires_reset() {
        assert!(VirtioError::InitFailed.requires_reset());
        assert!(!VirtioError::QueueFull.requires_reset());
    }

    #[test]
    fn test_blk_error_conversion() {
        let virtio_err = VirtioError::DmaError;
        let blk_err: BlkError = virtio_err.into();
        assert_eq!(blk_err, BlkError::VirtioError(VirtioError::DmaError));
    }
}