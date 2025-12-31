//! Debugging utilities for the bootloader
//! This module provides various debugging methods that work without serial ports

use core::fmt;
use core::ptr;
use core::arch::asm;

/// Debug buffer address - memory location for debug markers
pub const DEBUG_BUFFER_ADDRESS: usize = 0x8000_0000;
pub const DEBUG_BUFFER_SIZE: usize = 1024; // 1KB debug buffer

/// Debug marker types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugMarker {
    // Startup sequence
    BootloaderStart = 0xDEAD_BEEF,
    StackInitialized = 0xCAFE_BABE,
    DeviceInitialized = 0xFACE_B00C,
    // Loading stages
    KernelLoadingStart = 0x1000_0001,
    KernelLoadingProgress = 0x1000_0002,
    KernelLoadingComplete = 0x1000_0003,
    // ELF parsing
    ElfParseStart = 0x2000_0001,
    ElfParseSuccess = 0x2000_0002,
    ElfParseFailed = 0x2000_0003,
    // Memory operations
    SegmentLoadStart = 0x3000_0001,
    SegmentLoadSuccess = 0x3000_0002,
    SegmentLoadFailed = 0x3000_0003,
    // Final stages
    JumpToKernel = 0x4000_0001,
    PanicOccurred = 0xDEAD_DEAD,
}

/// Performance counter using RISC-V cycle CSR
pub struct PerfCounter {
    start_cycle: u64,
    description: &'static str,
}

impl PerfCounter {
    /// Create a new performance counter
    pub fn new(description: &'static str) -> Self {
        let start = Self::read_cycle();
        Self {
            start_cycle: start,
            description,
        }
    }
    
    /// Get elapsed cycles
    pub fn elapsed(&self) -> u64 {
        Self::read_cycle().wrapping_sub(self.start_cycle)
    }
    
    /// Read the RISC-V cycle CSR
    fn read_cycle() -> u64 {
        let mut cycle: u64;
        unsafe {
            asm!("csrr {}, cycle", out(reg) cycle);
        }
        cycle
    }
    
    /// Print elapsed time (in cycles)
    pub fn print_elapsed(&self) {
        let elapsed = self.elapsed();
        debug_write_u64(DEBUG_BUFFER_ADDRESS + 0x20, elapsed);
        // Also store description pointer (simplified)
        debug_write_str(DEBUG_BUFFER_ADDRESS + 0x30, self.description);
    }
}

/// Write a 32-bit value to debug memory
pub fn debug_write_u32(addr: usize, value: u32) {
    unsafe {
        ptr::write_volatile(addr as *mut u32, value);
    }
}

/// Write a 64-bit value to debug memory
pub fn debug_write_u64(addr: usize, value: u64) {
    unsafe {
        ptr::write_volatile(addr as *mut u64, value);
    }
}

/// Write a string to debug memory (max 256 chars)
pub fn debug_write_str(addr: usize, s: &str) {
    let max_len = 256;
    let bytes = s.as_bytes();
    let len = bytes.len().min(max_len);
    
    unsafe {
        // Write length first
        ptr::write_volatile(addr as *mut u32, len as u32);
        // Write string bytes
        for i in 0..len {
            ptr::write_volatile((addr + 4 + i) as *mut u8, bytes[i]);
        }
        // Null terminate
        if len < max_len {
            ptr::write_volatile((addr + 4 + len) as *mut u8, 0);
        }
    }
}

/// Set a debug marker (stage of boot process)
pub fn set_debug_marker(marker: DebugMarker) {
    debug_write_u32(DEBUG_BUFFER_ADDRESS, marker as u32);
    // Also store timestamp
    let cycle = PerfCounter::read_cycle();
    debug_write_u64(DEBUG_BUFFER_ADDRESS + 0x08, cycle);
}

/// Debug buffer structure (mapped to memory)
#[repr(C)]
pub struct DebugBuffer {
    pub magic: u32,        // Magic number: 0xDEADBEEF
    pub stage_marker: u32, // Current stage marker
    pub error_code: u32,   // Error code if any
    pub cycle_count: u64,  // Cycle counter value
    pub data1: u64,        // General purpose data 1
    pub data2: u64,        // General purpose data 2
    pub message: [u8; 256], // Debug message string
}

impl DebugBuffer {
    /// Initialize the debug buffer
    pub fn init() -> &'static mut Self {
        let buffer = unsafe { &mut *(DEBUG_BUFFER_ADDRESS as *mut DebugBuffer) };
        buffer.magic = 0xDEAD_BEEF;
        buffer.stage_marker = 0;
        buffer.error_code = 0;
        buffer.cycle_count = 0;
        buffer.data1 = 0;
        buffer.data2 = 0;
        // Clear message
        for i in 0..buffer.message.len() {
            buffer.message[i] = 0;
        }
        buffer
    }
    
    /// Set a debug message
    pub fn set_message(&mut self, msg: &str) {
        let bytes = msg.as_bytes();
        let len = bytes.len().min(self.message.len() - 1);
        for i in 0..len {
            self.message[i] = bytes[i];
        }
        self.message[len] = 0; // Null terminate
    }
    
    /// Set an error
    pub fn set_error(&mut self, code: u32, msg: &str) {
        self.error_code = code;
        self.set_message(msg);
    }
}