// library/rustsbi/src/kernel_loader.rs
//! 向后兼容的内核加载器

use crate::kernel::{self};

/// 向后兼容的错误类型
pub use kernel::KernelError as LoaderError;

pub fn find_and_load_kernel() -> Result<(), LoaderError> {
    // 调用新模块的实现
    kernel::find_and_load_kernel().map_err(|e| e.into())
}
// 导出打印函数用于兼容性
pub use crate::kernel::util::{
    print, print_char, print_hex, print_uint, print_hex32, print_bool, print_hex64
};