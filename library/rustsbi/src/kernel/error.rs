// library/rustsbi/src/kernel/error.rs
use crate::virtio::blk::BlkError;

/// 内核加载错误类型
#[derive(Debug, Clone, Copy)]
pub enum KernelError {
    DeviceNotFound,
    DeviceInitFailed,
    FileSystemError(&'static str),
    KernelNotFound,
    ReadError,
    OutOfMemory,
    VirtioError(BlkError),
    ElfError(&'static str),
    InvalidFormat,
    SegmentLoadError,
    FsError(&'static str), 
    InitFailed,      // 设备初始化失败
    IoError,         // IO操作错误
    BufferTooSmall,  // 缓冲区太小
}

impl From<BlkError> for KernelError {
    fn from(err: BlkError) -> Self {
        KernelError::VirtioError(err)
    }
}

impl core::fmt::Display for KernelError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            KernelError::DeviceNotFound => write!(f, "Device not found"),
            KernelError::DeviceInitFailed => write!(f, "Device initialization failed"),
            KernelError::FileSystemError(msg) => write!(f, "Filesystem error: {}", msg),
            KernelError::KernelNotFound => write!(f, "Kernel not found"),
            KernelError::ReadError => write!(f, "Read error"),
            KernelError::OutOfMemory => write!(f, "Out of memory"),
            KernelError::VirtioError(e) => write!(f, "Virtio error: {:?}", e),
            KernelError::ElfError(msg) => write!(f, "ELF error: {}", msg),
            KernelError::InvalidFormat => write!(f, "Invalid format"),
            KernelError::SegmentLoadError => write!(f, "Segment load error"),
	    KernelError::FsError(msg) => write!(f, "Filesystem error: {}", msg),
            KernelError::InitFailed => write!(f, "设备初始化错误"),
            KernelError::IoError => write!(f, "IO操作错误"),
	    KernelError::BufferTooSmall => write!(f, "缓冲区太小"),
        
        }
    }
}