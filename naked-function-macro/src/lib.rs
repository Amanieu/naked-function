//! Implementation of the proc macro used by the `naked-function` crate.
//!
//! Don't use this crate directly, use the `naked-function` crate instead.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

use quote::ToTokens;
use syn::{parse::Nothing, parse_macro_input};

macro_rules! bail {
    ($span:expr, $($tt:tt)*) => {
        return Err(syn::Error::new_spanned($span, format!($($tt)*)))
    };
}

mod asm;
mod naked;

/// An attribute to define a function written entirely in assembly.
///
/// A naked function must contain only a single `asm!` statement: the contents
/// of this `asm!` becomes the body of the function, with no prologue or
/// epilogue. This means that the assembly code is responsible for including
/// the necessary instructions to return from a function.
///
/// The primary use of naked function is to implement functions that use a
/// custom calling convention that is not directly supported by rustc. Examples
/// include hardware exception handlers, functions called from assembly code
/// and customizing unwinding metadata.
///
/// ## Example
///
/// ```rust
/// # #![cfg(all(target_os = "linux", target_arch = "x86_64"))]
/// // The SYSV64 calling convention used on x86_64 Linux passes the first
/// // 2 integer arguments in EDI/ESI.
/// #[naked_function::naked]
/// pub unsafe extern "C" fn add(a: i32, b: i32) -> i32 {
///     asm!(
///         "lea eax, [edi + esi]",
///         "ret",
///     );
/// }
///
/// #[test]
/// fn main() {
///     let ret = unsafe { add(1, 2) };
///     assert_eq!(ret, 3);
/// }
/// ```
///
/// ## `asm!` restrictions
///
/// The `asm!` is further restricted in that:
/// - Only `sym` and `const` operands are allowed.
/// - `clobber_abi` cannot be used.
/// - Only the `raw` and `att_syntax` options can be used.
///
/// These are the same set of operands accepted by `global_asm!`, which this
/// attribute lowers the functions into.
///
/// ## Accessing function arguments.
///
/// The function signature is indicative only: it is merely there to allow
/// Rust code to reference and call the naked function.
///
/// Function arguments cannot be referenced from the function body directly,
/// instead you should access these from the expected register/stack slot as
/// per the function ABI.
///
/// Similarly, you are responsible for placing function return values in the
/// appropriate registers or stack slot for the calling convention used.
///
/// ## ABI and attributes
///
/// Naked functions must be marked as `unsafe`.
///
/// The function must have one of the following whitelisted ABIs:
/// - `"C"`
/// - `"C-unwind"`
///
/// Only the following attributes are supported on naked functions:
/// - `#[export_name]`
/// - `#[no_mangle]`
/// - `#[link_section]`
/// - `#[cfg]`
/// - `#[doc]` or `///` doc comments
#[proc_macro_attribute]
pub fn naked(attr: TokenStream, item: TokenStream) -> TokenStream {
    parse_macro_input!(attr as Nothing);
    match naked::naked_attribute(&parse_macro_input!(item)) {
        Ok((foreign_mod, global_asm)) => {
            let mut tokens = TokenStream2::new();
            foreign_mod.to_tokens(&mut tokens);
            global_asm.to_tokens(&mut tokens);
            tokens.into()
        }
        Err(e) => e.to_compile_error().into(),
    }
}
