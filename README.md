`#[naked]`
==========

[![Crates.io](https://img.shields.io/crates/v/naked-function.svg)](https://crates.io/crates/naked-function)

### [Documentation](https://docs.rs/naked-function/)

This crate provide a proc macro version of the `#[naked]` attribute which can
be used on stable Rust.

## Example


```rust
// The SYSV64 calling convention used on x86_64 Linux passes the first
// 2 integer arguments in EDI/ESI.
#[naked_function::naked]
pub unsafe extern "C" fn add(a: i32, b: i32) -> i32 {
    asm!(
        "lea eax, [edi + esi]",
        "ret",
    );
}

#[test]
fn main() {
    let ret = unsafe { add(1, 2) };
    assert_eq!(ret, 3);
}
```

## License

Licensed under either of:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
