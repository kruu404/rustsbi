// library/rustsbi/src/kernel/mod.rs
//! 内核加载模块

// 子模块
pub mod error;
pub mod elf_parser;
//pub mod fs;
pub mod boot;
pub mod loader;
pub mod util;
pub mod boot_env;
pub mod memory_layout;
pub mod debug;

// 类型重导出
pub use error::KernelError;
pub use elf_parser::ElfParser;
//pub use fs::{FileSystemManager, FilesystemType, SimpleFs};
pub use boot::BootConfig;
pub use loader::KernelLoader;
pub use util::{print, print_char, print_hex, print_uint, print_hex32, print_bool, print_hex64};

use crate::kernel::boot_env::boot_kernel;

/// 🛠️ 修改后的主加载函数 - 返回加载状态而不是缓冲区
pub fn find_and_load_kernel() -> Result<(), KernelError> {
    let blk_device = crate::virtio::blk::VirtioBlk::probe_all_devices()
        .ok_or(KernelError::DeviceNotFound)?;
    
    let mut loader = KernelLoader::new(blk_device);
    
    // 🛠️ 调用加载方法，成功即返回Ok(())
    loader.find_and_load_kernel()?;
    
    // 🆕 成功加载后直接返回，缓冲区数据通过其他方式访问
    Ok(())
}

/// 🆕 保持创建加载器的方法
pub fn create_kernel_loader() -> Result<KernelLoader, KernelError> {
    let blk_device = crate::virtio::blk::VirtioBlk::probe_all_devices()
        .ok_or(KernelError::DeviceNotFound)?;
    
    Ok(KernelLoader::new(blk_device))
}