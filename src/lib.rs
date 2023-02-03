//! This crate provides the [`naked`] proc macro.
#![no_std]

#[doc(inline)]
pub use naked_function_macro::naked;

// Helper macros to deal with platform-specific differences in assembly code
// between ELF, Mach-O and COFF file formats.
//
// We can't do this within the proc macro itself because Rust don't expose the
// target cfgs to proc macros.

cfg_if::cfg_if! {
    if #[cfg(any(
            target_vendor = "apple",
            all(windows, target_arch = "x86"),
        ))] {
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_mangle {
            ($symbol:expr) => { concat!("_", $symbol) };
        }
    } else {
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_mangle {
            ($symbol:expr) => { $symbol };
        }
    }
}
cfg_if::cfg_if! {
    if #[cfg(windows)] {
        // COFF
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_function_begin {
            ($symbol:expr, $section:expr) => {
                concat!(
                    ".pushsection ", $section, ",\"xr\"\n",
                    ".balign 4\n",
                    ".globl ", $crate::__asm_mangle!($symbol), "\n",
                    ".def ", $crate::__asm_mangle!($symbol), "\n",
                    ".scl 2\n",
                    ".type 32\n",
                    ".endef ", $crate::__asm_mangle!($symbol), "\n",
                    $crate::__asm_mangle!($symbol), ":\n",
                )
            };
        }
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_function_end {
            ($symbol:expr) => {
                ".popsection"
            };
        }
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_default_section {
            ($symbol:expr) => { concat!(".text.", $symbol) };
        }
    } else if #[cfg(target_vendor = "apple")] {
        // Mach-O
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_function_begin {
            ($symbol:expr, $section:expr) => {
                concat!(
                    ".pushsection ", $section, ",regular,pure_instructions\n",
                    ".balign 4\n",
                    ".globl ", $crate::__asm_mangle!($symbol), "\n",
                    ".private_extern ", $crate::__asm_mangle!($symbol), "\n",
                    $crate::__asm_mangle!($symbol), ":\n",
                )
            };
        }
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_function_end {
            ($symbol:expr) => {
                ".popsection"
            };
        }
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_default_section {
            ($symbol:expr) => { "__TEXT,__text" };
        }
    } else {
        // Everything else uses ELF. ARM uses % instead of @ for some
        // assembler directives.
        #[cfg(not(target_arch = "arm"))]
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_type {
            ($ty:literal) => { concat!("@", $ty) }
        }
        #[cfg(target_arch = "arm")]
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_function_type {
            ($ty:literal) => { concat!("%", $ty) }
        }
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_function_begin {
            ($symbol:expr, $section:expr) => {
                concat!(
                    ".pushsection ", $section, ",\"ax\", ", $crate::__asm_type!("progbits"), "\n",
                    ".balign 4\n",
                    ".globl ", $crate::__asm_mangle!($symbol), "\n",
                    ".hidden ", $crate::__asm_mangle!($symbol), "\n",
                    ".type ", $crate::__asm_mangle!($symbol), ", ", $crate::__asm_type!("function"), "\n",
                    $crate::__asm_mangle!($symbol), ":\n",
                )
            };
        }
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_function_end {
            ($symbol:expr) => {
                concat!(
                    ".size ", $crate::__asm_mangle!($symbol), ", . - ", $crate::__asm_mangle!($symbol), "\n",
                    ".popsection"
                )
            };
        }
        #[doc(hidden)]
        #[macro_export]
        macro_rules! __asm_default_section {
            ($symbol:expr) => { concat!(".text.", $symbol) };
        }
    }
}
