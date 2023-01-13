#![cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#![cfg(target_os = "linux")]

#[naked_function::naked]
#[cfg(target_os = "linux")]
#[cfg(target_arch = "x86_64")]
pub unsafe extern "C" fn ret() -> i32 {
    asm!("mov rax, 1", "ret");
}

#[naked_function::naked]
#[cfg(target_arch = "aarch64")]
#[cfg(target_os = "linux")]
pub unsafe extern "C" fn ret() -> i32 {
    asm!("mov x0, 2", "ret");
}

#[test]
fn test_conditional() {
    let x = unsafe { ret() };

    if cfg!(x86_64) {
        debug_assert_eq!(x, 1);
    } else if cfg!(aarch64) {
        debug_assert_eq!(x, 2);
    }
}
