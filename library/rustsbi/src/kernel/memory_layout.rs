//! Memory layout definitions for RISC-V bootloader
#![allow(dead_code)]

/// Kernel entry point address
pub const KERNEL_LOAD_ADDRESS: usize = 0x8020_0000;

/// Stack configuration
pub const KERNEL_STACK_SIZE: usize = 64 * 1024; // 64KB
pub const KERNEL_STACK_TOP: usize = 0x8100_0000;

/// Device tree blob address
pub const DEVICE_TREE_ADDRESS: usize = 0x8200_0000;

/// VirtIO MMIO base address
pub const VIRTIO_MMIO_BASE: usize = 0x1000_1000;

/// Kernel buffer for loading
pub const KERNEL_BUFFER_ADDRESS: usize = 0x8000_0000;
pub const KERNEL_BUFFER_SIZE: usize = 1024 * 1024; // 1MB

/// Disk layout
pub const KERNEL_START_BLOCK: u64 = 1; // Kernel starts at block 1
pub const BLOCK_SIZE: usize = 512;     // Standard block size