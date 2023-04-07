#![cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#![cfg(target_os = "linux")]

#[naked_function::naked]
#[cfg(target_os = "linux")]
#[cfg(target_arch = "x86_64")]
#[rustfmt::skip]
pub unsafe extern "C" fn ret() -> i32 {
    asm!(
        "mov rax, 1",
        "ret",
    );
}

#[naked_function::naked]
#[cfg(target_arch = "aarch64")]
#[cfg(target_os = "linux")]
#[rustfmt::skip]
pub unsafe extern "C" fn ret() -> i32 {
    asm!(
        "mov x0, 2",
        "ret",
    );
}
