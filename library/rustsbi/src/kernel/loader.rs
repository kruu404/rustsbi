// library/rustsbi/src/kernel/loader.rs
use super::error::KernelError;
use crate::virtio::blk::VirtioBlk;
use super::util::{print, print_uint};
use heapless::String;

const SAFE_BUFFER_BASE: usize = 0x81000000; // 确保这个地址远离内核区域
const BUFFER_SIZE: usize = 0x100000; // 1MB

// 从链接脚本引入符号，这些符号由link.ld定义
unsafe extern "C" {
    static _buffer_start: u8;
    static _buffer_end: u8;
}

/// 改进后的内核加载器 - 支持智能ELF检测和跳过空数据
pub struct KernelLoader {
    blk_device: VirtioBlk,
    device_initialized: bool,
    //buffer: Vec<u8, 1007616>,
    elf_start_sector: Option<u32>, // 🆕 记录ELF起始扇区
    bytes_loaded: usize, // 新增：记录实际加载了多少字节
}

// 进度条辅助结构保持不变
struct ProgressBar;

impl ProgressBar {
    pub fn new(_total: usize) -> Self {
        print("\r🔄 Progress: [--------------------] 0%");
        ProgressBar
    }

    pub fn update(&mut self, current: usize, total: usize, update_interval: usize) {
        if current % update_interval == 0 || current + 1 == total {
            let percent = (current as f32 / total as f32) * 100.0;
            let filled_length = (percent / 5.0) as usize;

            let mut bar_chars = [' '; 20];
            for i in 0..filled_length {
                if i < 19 {
                    bar_chars[i] = '=';
                } else {
                    bar_chars[i] = '>';
                }
            }
            
            let mut bar = String::<40>::new();
            bar.push_str("[").unwrap();
            for i in 0..20 {
                bar.push(bar_chars[i]).unwrap();
            }
            bar.push_str("]").unwrap();

            print("\r🔄 读取进度: ");
            print(&bar);
            print(" ");
            print_uint(percent as u32);
            print("%");
        }
    }
}

impl KernelLoader {
    pub fn new(blk_device: VirtioBlk) -> Self {
        Self { 
            blk_device,
            device_initialized: false,
            //buffer: Vec::new(),
            elf_start_sector: None, // 🆕 初始化ELF起始扇区
	    bytes_loaded: 0,
        }
    }
  
    /// 🆕 新增：智能ELF检测函数
    fn detect_elf_start_sector(&mut self) -> Result<u32, KernelError> {
        
        // 先检查扇区0（传统位置）
        let mut sector_data = [0u8; 512];
        if let Ok(()) = self.blk_device.read_block(0, &mut sector_data) {
            if Self::is_elf_signature(&sector_data) {
                return Ok(0);
            }
        }
        
        // 从扇区1开始搜索（跳过可能的引导扇区）
        for sector in 1..100 { // 搜索前100个扇区
            let mut sector_data = [0u8; 512];
            match self.blk_device.read_block(sector, &mut sector_data) {
                Ok(()) => {
                    if Self::is_elf_signature(&sector_data) {
                        print("🎯 ELF发现在扇区 ");
                        print_uint(sector.try_into().unwrap());
                        print("\r\n");
                        return Ok(sector.try_into().unwrap());
                    }
                }
                Err(_) => {
                    // 遇到读取错误时继续搜索下一个扇区
                    continue;
                }
            }
        }
        
        // 如果没找到，回退到扇区1（常见位置）
        print("⚠️  No ELF signature found, defaulting to sector 1\r\n");
        Ok(1)
    }
    
    /// 🆕 新增：检查是否为ELF签名
    fn is_elf_signature(data: &[u8]) -> bool {
        data.len() >= 4 && 
        data[0] == 0x7F && 
        data[1] == b'E' && 
        data[2] == b'L' && 
        data[3] == b'F'
    }

/// 🆕 新增：调试功能 - 显示缓冲区每个扇区的前64字节数据
/*
    fn debug_buffer_sectors(&self, buffer_start_addr: usize, sectors_to_read: u32) {
        print("\r\n🔍 开始调试缓冲区内容（每个扇区前64字节）:\r\n");
        
        for sector_offset in 0..sectors_to_read {
            let offset_in_buffer = sector_offset as usize * 512;
            
            // 确保不会越界访问
            if offset_in_buffer + 64 > self.bytes_loaded {
                break;
            }
            
            print("📍 扇区 ");
            print_uint(sector_offset);
            print(" (偏移 0x");
            print_hex32(offset_in_buffer as u32);
            print("): ");
            
            // 读取当前扇区的前64字节
            unsafe {
                let sector_ptr = (buffer_start_addr + offset_in_buffer) as *const u8;
                
                // 显示十六进制数据
                for i in 0..64 {
                    if i % 16 == 0 && i > 0 {
                        print("\r\n                    "); // 对齐显示
                    }
                    print_hex32((*sector_ptr.add(i)) as u32);
                    print(" ");
                }
            }
            print("\r\n");
            
            // 检查当前扇区开头是否有ELF签名
            let has_elf_signature = unsafe {
                let sig_ptr = (buffer_start_addr + offset_in_buffer) as *const u8;
                (*sig_ptr == 0x7F) && 
                (*sig_ptr.add(1) == b'E') && 
                (*sig_ptr.add(2) == b'L') && 
                (*sig_ptr.add(3) == b'F')
            };
            
            if has_elf_signature {
                print("   ✅ 发现ELF签名!\r\n");
            }
            
            // 每显示几个扇区后暂停一下，避免输出过多
            if sector_offset % 5 == 4 {
                self.delay(1000); // 稍微延迟以便观察
            }
        }
        
        print("\r\n📊 调试完成，共检查 ");
        print_uint(sectors_to_read as u32);
        print(" 个扇区，总字节数: ");
        print_uint(self.bytes_loaded as u32);
        print("\r\n");
    }
*/
    
    /// 🛠️ 改进后的核心加载函数 - 从ELF位置开始读取
    pub fn load_kernel_raw(&mut self) -> Result<(), KernelError> {
        // 1. 初始化设备
        if !self.device_initialized {          
            if let Err(_) = self.blk_device.initialize() {
                print("❌ Device initialization failed\r\n");
                return Err(KernelError::InitFailed);
            }
            self.device_initialized = true;
        } 
        
        // 2. 🆕 检测ELF起始扇区
        let start_sector = match self.detect_elf_start_sector() {
            Ok(sector) => sector,
            Err(e) => {
                print("❌ ELF detection failed\r\n");
                return Err(e);
            }
        };
        
        self.elf_start_sector = Some(start_sector);
        
        // 3. 清空缓冲区
        //self.buffer.clear();
        
        // 4. 计算需要读取的扇区数量
        let sectors_to_read = 1968u32.saturating_sub(start_sector); // 确保不溢出
        if sectors_to_read == 0 {
            print("❌ No sectors to read after ELF detection\r\n");
            return Err(KernelError::IoError);
        }
        
        print("📖 读取扇区 ");
        print_uint(start_sector);
        print("-");
        print_uint(start_sector + sectors_to_read - 1);
        print(" (");
        print_uint((sectors_to_read * 512) as u32);
        print(" 字节)\r\n");

        // 5. 初始化进度条
        let total = sectors_to_read as usize;
        let update_interval = (total / 50).max(1);
        let mut progress_bar = ProgressBar::new(total);

	 // 清空旧的长度记录
        self.bytes_loaded = 0;

let buffer_start_addr = SAFE_BUFFER_BASE;
let buffer_size = BUFFER_SIZE;

        // 扇区读取循环
        for sector_offset in 0..sectors_to_read {
            let actual_sector = start_sector + sector_offset;
            let mut sector_data = [0u8; 512];
            
            match self.blk_device.read_block(actual_sector.into(), &mut sector_data) {
                Ok(()) => {               
                    // 计算当前扇区在外部缓冲区中的偏移
                    let offset_in_buffer = sector_offset as usize * 512;
                    
                    // 🛠️ 关键修改：添加缓冲区边界检查
                    if offset_in_buffer + 512 > buffer_size {
                        print("❌ 缓冲区空间不足，无法读取更多扇区\r\n");
                        return Err(KernelError::BufferTooSmall);
                    }

                    // 🛠️ 关键修改：将数据直接拷贝到外部缓冲区
                    unsafe {
                        let target_ptr = (buffer_start_addr + offset_in_buffer) as *mut u8;
                        core::ptr::copy_nonoverlapping(
                            sector_data.as_ptr(), 
                            target_ptr, 
                            512
                        );
                    }
                    
                    self.bytes_loaded = offset_in_buffer + 512; // 更新有效数据长度
                    
                    // 更新进度条
                    progress_bar.update(sector_offset as usize, total, update_interval);
                }
                Err(_) => {
                    print("❌ 失败读取扇区 ");
                    print_uint(actual_sector);
                    print("\r\n");
                    return Err(KernelError::IoError);
                }
            }
            self.delay(100);
        }

        print("\r\n");

// 🆕 调用调试功能显示缓冲区内容
        //self.debug_buffer_sectors(buffer_start_addr, sectors_to_read);
        
        // 🛠️ 修改验证逻辑：使用外部缓冲区中的数据验证ELF签名
        let elf_signature_valid = unsafe {
            let sig_ptr = buffer_start_addr as *const u8;
            // 检查缓冲区开头4个字节是否为ELF签名
            (*sig_ptr == 0x7F) && 
            (*sig_ptr.add(1) == b'E') && 
            (*sig_ptr.add(2) == b'L') && 
            (*sig_ptr.add(3) == b'F')
        };

        if elf_signature_valid {
            print("✅ 内核成功加载到缓冲区！\r\n");
        } else {
            print("⚠️  WARNING: Expected ELF signature not found in external buffer\r\n");
        }
        
        Ok(())
    }
    
    /// 🆕 新增：获取ELF起始扇区信息
    pub fn get_elf_start_sector(&self) -> Option<u32> {
        self.elf_start_sector
    }
    
    /// 🆕 新增：获取缓冲区中ELF数据的实际偏移量
pub fn get_elf_data_with_offset(&self) -> (&[u8], usize) {
    unsafe {
        // 使用与磁盘读取和调试函数完全相同的 SAFE_BUFFER_BASE
        let buffer_ptr = SAFE_BUFFER_BASE as *const u8;
        let buffer_slice = core::slice::from_raw_parts(buffer_ptr, self.bytes_loaded);
        (buffer_slice, 0)
    }
}

    pub fn find_and_load_kernel(&mut self) -> Result<(), KernelError> {
        self.load_kernel_raw()
    }
    
    fn delay(&self, cycles: u32) {
        unsafe {
            for _ in 0..cycles {
                core::arch::asm!("nop");
            }
        }
    }
}