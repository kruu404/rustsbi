// library/rustsbi/src/kernel/boot.rs
use super::error::KernelError;

/// 引导配置参数
#[derive(Debug, Clone, Copy)]
pub struct BootConfig {
    pub kernel_entry: u64,
    pub memory_start: u64,
    pub memory_size: u64,
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            kernel_entry: 0x8020_0000,
            memory_start: 0x8000_0000,
            memory_size: 128 * 1024 * 1024,
        }
    }
}

impl BootConfig {
    pub fn from_kernel_info(entry_point: u64, _kernel_size: usize) -> Result<Self, KernelError> {
        let mut config = BootConfig::default();
        config.kernel_entry = entry_point;
        Ok(config)
    }
    
    pub fn validate(&self) -> Result<(), KernelError> {
        if self.kernel_entry < self.memory_start {
            return Err(KernelError::ElfError("Invalid kernel entry point"));
        }
        Ok(())
    }
}