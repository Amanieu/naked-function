#![cfg(target_arch = "aarch64")]
#![feature(asm_const)]

#[naked_function::naked]
pub unsafe extern "C" fn add(a: i32, b: i32) -> i32 {
    asm!("add w0, w0, w1", "ret");
}

#[test]
fn test_add() {
    let ret = unsafe { add(1, 2) };
    assert_eq!(ret, 3);
}

#[naked_function::naked]
pub unsafe extern "C" fn add_5(a: i32) -> i32 {
    asm!(
        "add w0, w0, {}",
        "ret",
        const 5,
    )
}

#[test]
fn test_const() {
    let ret = unsafe { add_5(3) };
    assert_eq!(ret, 8);
}

extern "C" fn mutate_string(str: &mut String) {
    assert_eq!(str, "hello");
    *str = "world".into();
}

#[allow(improper_ctypes)]
#[naked_function::naked]
pub unsafe extern "C" fn call_sym(str: &mut String) -> i32 {
    asm!(
        "str lr, [sp, #-16]!",
        "bl {}",
        "ldr lr, [sp], #16",
        "ret",
        sym mutate_string
    );
}

#[test]
fn test_sym() {
    let mut str = "hello".to_string();
    unsafe {
        call_sym(&mut str);
    }
    assert_eq!(str, "world");
}

#[naked_function::naked]
#[export_name = "exported_symbol_name"]
pub unsafe extern "C" fn export_name() -> i32 {
    asm!("mov x0, 3", "ret");
}

#[test]
fn test_export_name() {
    extern "C" {
        fn exported_symbol_name() -> i32;
    }
    let val = unsafe { exported_symbol_name() };
    assert_eq!(val, 3);
}

#[naked_function::naked]
pub unsafe extern "C" fn mangled() -> i32 {
    asm!("mov x0, 4", "ret");
}

mod scoped {
    #[no_mangle]
    fn mangled() -> i32 {
        5
    }
}

#[test]
fn test_mangled() {
    extern "C" {
        fn mangled() -> i32;
    }
    let val = unsafe { mangled() };
    assert_eq!(val, 5);
}
