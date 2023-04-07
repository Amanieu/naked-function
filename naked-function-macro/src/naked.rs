use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{
    punctuated::Punctuated, Abi, AttrStyle, Attribute, Expr, ExprLit, ExprMacro, ForeignItem,
    ForeignItemFn, Item, ItemFn, ItemForeignMod, ItemMacro, Lit, LitStr, Macro, MacroDelimiter,
    Meta, MetaNameValue, Result, Signature, Token,
};

use crate::asm::{extract_asm, AsmOperand};

/// Sanity checks the function signature.
fn validate_sig(sig: &Signature) -> Result<()> {
    if let Some(constness) = sig.constness {
        bail!(constness, "#[naked] is not supported on const functions");
    }
    if let Some(asyncness) = sig.asyncness {
        bail!(asyncness, "#[naked] is not supported on async functions");
    }
    if sig.unsafety.is_none() {
        bail!(sig, "#[naked] can only be used on unsafe functions");
    }
    match &sig.abi {
        Some(Abi {
            extern_token: _,
            name: Some(name),
        }) if matches!(&*name.value(), "C" | "C-unwind") => {}
        _ => bail!(
            &sig.abi,
            "#[naked] functions must be `extern \"C\"` or `extern \"C-unwind\"`"
        ),
    }
    if !sig.generics.params.is_empty() {
        bail!(
            &sig.generics,
            "#[naked] cannot be used with generic functions"
        );
    }
    Ok(())
}

struct ParsedAttrs {
    foreign_attrs: Vec<Attribute>,
    cfg: Vec<Attribute>,
    symbol: Expr,
    link_section: Expr,
}

/// Parses the attributes on the function and checks them against a whitelist
/// of supported attributes.
///
/// The symbol name of the function and the linker section it will be placed in
/// are computed here based on the function attributes.
fn parse_attrs(ident: &Ident, attrs: &[Attribute]) -> Result<ParsedAttrs> {
    let mut foreign_attrs = vec![];
    let mut cfg = vec![];
    let mut no_mangle = false;
    let mut export_name = None;
    let mut link_section = None;

    // Attributes to forward to the foreign function declaration that we will
    // generate.
    let attr_whitelist = [
        "doc",
        "allow",
        "warn",
        "deny",
        "forbid",
        "deprecated",
        "must_use",
    ];

    'outer: for attr in attrs {
        if let AttrStyle::Inner(_) = attr.style {
            bail!(attr, "unexpected inner attribute");
        }

        // Forward whitelisted attributes to the foreign item.
        for whitelist in attr_whitelist {
            if attr.path().is_ident(whitelist) {
                foreign_attrs.push(attr.clone());
                continue 'outer;
            }
        }

        if attr
            .path()
            .segments
            .first()
            .map_or(false, |segment| segment.ident == "rustfmt")
        {
            // Ignore rustfmt attributes
        } else if attr.path().is_ident("no_mangle") {
            attr.meta.require_path_only()?;
            no_mangle = true;
        } else if attr.path().is_ident("export_name") {
            // Pass the export_name attribute through as a #[link_section] on
            // the foreign import declaration.
            let name_value = attr.meta.require_name_value()?;
            export_name = Some(name_value.value.clone());
            let mut link_name = attr.clone();
            link_name.meta = Meta::NameValue(MetaNameValue {
                path: syn::parse2(quote!(link_name)).unwrap(),
                eq_token: name_value.eq_token,
                value: name_value.value.clone(),
            });
            foreign_attrs.push(link_name);
        } else if attr.path().is_ident("link_section") {
            let name_value = attr.meta.require_name_value()?;
            link_section = Some(name_value.value.clone());
        } else if attr.path().is_ident("cfg") {
            cfg.push(attr.clone())
        } else {
            bail!(
                attr,
                "naked functions only support \
                #[no_mangle], #[export_name] and #[link_section] attributes"
            );
        }
    }

    let symbol = if let Some(export_name) = &export_name {
        export_name.clone()
    } else {
        let raw_symbol = if no_mangle {
            ident.to_string()
        } else {
            format!("rust_naked_function_{}", ident.to_string())
        };

        Expr::Lit(ExprLit {
            attrs: vec![],
            lit: Lit::Str(LitStr::new(&raw_symbol, Span::call_site())),
        })
    };

    // Add a #[link_name] attribute to the import pointing to our manually
    // mangled symbol name.
    if export_name.is_none() {
        foreign_attrs.push(Attribute {
            pound_token: Default::default(),
            style: AttrStyle::Outer,
            bracket_token: Default::default(),
            meta: Meta::NameValue(MetaNameValue {
                path: syn::parse2(quote!(link_name)).unwrap(),
                eq_token: Default::default(),
                value: symbol.clone(),
            }),
        });
    }

    // Use the given section if provided, otherwise use the platform
    // default. This is usually .text.$SYMBOL, except on Mach-O targets
    // which don't have per-symbol sections.
    let link_section = if let Some(link_section) = link_section {
        link_section
    } else {
        Expr::Macro(ExprMacro {
            attrs: vec![],
            mac: Macro {
                path: syn::parse2(quote!(::naked_function::__asm_default_section)).unwrap(),
                bang_token: Default::default(),
                delimiter: MacroDelimiter::Paren(Default::default()),
                tokens: symbol.to_token_stream(),
            },
        })
    };

    Ok(ParsedAttrs {
        foreign_attrs,
        cfg,
        symbol,
        link_section,
    })
}

fn emit_foreign_mod(func: &ItemFn, attrs: &ParsedAttrs) -> ItemForeignMod {
    // Remove the ABI and unsafe from the function signature and move it to the
    // `extern` block.
    let sig = Signature {
        abi: None,
        unsafety: None,
        ..func.sig.clone()
    };
    let foreign_fn = ForeignItem::Fn(ForeignItemFn {
        attrs: {
            let mut attrs_ = attrs.foreign_attrs.clone();
            attrs_.extend_from_slice(&attrs.cfg[..]);
            attrs_
        },
        vis: func.vis.clone(),
        sig,
        semi_token: Default::default(),
    });
    ItemForeignMod {
        attrs: vec![],
        unsafety: None,
        abi: func.sig.abi.clone().unwrap(),
        brace_token: Default::default(),
        items: vec![foreign_fn],
    }
}

fn emit_global_asm(attrs: &ParsedAttrs, mut asm: Punctuated<AsmOperand, Token![,]>) -> ItemMacro {
    // Inject a prefix to the assembly code containing the necessary assembler
    // directives to start a function.
    let mut prefix_args = Punctuated::<Expr, Token![,]>::new();
    prefix_args.push(attrs.symbol.clone());
    prefix_args.push(attrs.link_section.clone());
    let prefix = Expr::Macro(ExprMacro {
        attrs: vec![],
        mac: Macro {
            path: syn::parse2(quote!(::naked_function::__asm_function_begin)).unwrap(),
            bang_token: Default::default(),
            delimiter: MacroDelimiter::Paren(Default::default()),
            tokens: prefix_args.into_token_stream(),
        },
    });
    asm.insert(0, AsmOperand::Template(prefix));

    // Inject a suffix at the end of the assembly code containing assembler
    // directives to end a function.
    let last_template = asm
        .iter()
        .rposition(|op| matches!(op, AsmOperand::Template(_)))
        .unwrap();
    let suffix = Expr::Macro(ExprMacro {
        attrs: vec![],
        mac: Macro {
            path: syn::parse2(quote!(::naked_function::__asm_function_end)).unwrap(),
            bang_token: Default::default(),
            delimiter: MacroDelimiter::Paren(Default::default()),
            tokens: attrs.symbol.to_token_stream(),
        },
    });
    asm.insert(last_template + 1, AsmOperand::Template(suffix));

    let global_asm = Macro {
        path: syn::parse2(quote!(::core::arch::global_asm)).unwrap(),
        bang_token: Default::default(),
        delimiter: MacroDelimiter::Paren(Default::default()),
        tokens: asm.to_token_stream(),
    };
    ItemMacro {
        attrs: attrs.cfg.clone(),
        ident: None,
        mac: global_asm,
        semi_token: Some(Default::default()),
    }
}

/// Entry point of the proc macro.
pub fn naked_attribute(func: &ItemFn) -> Result<Vec<Item>> {
    validate_sig(&func.sig)?;
    let attrs = parse_attrs(&func.sig.ident, &func.attrs)?;
    let asm = extract_asm(func)?;
    let foreign_mod = emit_foreign_mod(func, &attrs);
    let global_asm = emit_global_asm(&attrs, asm);
    Ok(vec![Item::ForeignMod(foreign_mod), Item::Macro(global_asm)])
}
