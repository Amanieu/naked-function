use quote::ToTokens;
use syn::{
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Paren,
    Expr, Ident, ItemFn, Result, Stmt, Token,
};

pub mod kw {
    syn::custom_keyword!(sym);
    syn::custom_keyword!(options);
    syn::custom_keyword!(out);
    syn::custom_keyword!(lateout);
    syn::custom_keyword!(inout);
    syn::custom_keyword!(inlateout);
    syn::custom_keyword!(clobber_abi);
}

/// Representation of one argument of the `asm!` macro.
pub enum AsmOperand {
    Template(Expr),
    Const {
        name: Option<(Ident, Token![=])>,
        token: Token![const],
        expr: Expr,
    },
    Sym {
        name: Option<(Ident, Token![=])>,
        token: kw::sym,
        expr: Expr,
    },
    Options {
        token: kw::options,
        paren_token: Paren,
        options: Punctuated<Ident, Token![,]>,
    },
}

impl Parse for AsmOperand {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(kw::options) {
            let token = input.parse::<kw::options>()?;
            let content;
            let paren_token = parenthesized!(content in input);
            let options = content.parse_terminated(Ident::parse, Token![,])?;
            return Ok(Self::Options {
                token,
                paren_token,
                options,
            });
        }

        let mut name = None;
        if input.peek(Ident::peek_any) && input.peek2(Token![=]) {
            let ident = input.call(Ident::parse_any)?;
            let token = input.parse()?;
            name = Some((ident, token));
        }

        if input.peek(kw::sym) {
            let token = input.parse()?;
            let expr = input.parse()?;
            return Ok(Self::Sym { name, token, expr });
        }

        if input.peek(Token![const]) {
            let token = input.parse()?;
            let expr = input.parse()?;
            return Ok(Self::Const { name, token, expr });
        }

        if input.peek(Token![in])
            || input.peek(kw::out)
            || input.peek(kw::lateout)
            || input.peek(kw::inout)
            || input.peek(kw::inlateout)
        {
            return Err(syn::Error::new(
                input.span(),
                "only `const` and `sym` operands may be used in naked functions",
            ));
        }

        if input.peek(kw::clobber_abi) {
            return Err(syn::Error::new(
                input.span(),
                "`clobber_abi` cannot be used in naked functions",
            ));
        }

        // Assume anything else is a template string. global_asm! will properly
        // validate this for us later.
        if let Some((ident, _token)) = name {
            bail!(ident, "invalid asm! syntax");
        }
        Ok(Self::Template(input.parse()?))
    }
}

impl ToTokens for AsmOperand {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            AsmOperand::Template(expr) => expr.to_tokens(tokens),
            AsmOperand::Const { name, token, expr } => {
                if let Some((ident, token)) = name {
                    ident.to_tokens(tokens);
                    token.to_tokens(tokens);
                }
                token.to_tokens(tokens);
                expr.to_tokens(tokens);
            }
            AsmOperand::Sym { name, token, expr } => {
                if let Some((ident, token)) = name {
                    ident.to_tokens(tokens);
                    token.to_tokens(tokens);
                }
                token.to_tokens(tokens);
                expr.to_tokens(tokens);
            }
            AsmOperand::Options {
                token,
                paren_token,
                options,
            } => {
                token.to_tokens(tokens);
                paren_token.surround(tokens, |tokens| {
                    options.to_tokens(tokens);
                })
            }
        }
    }
}

/// Extracts the `AsmOperand`s from the `asm!` in the body of the function.
pub fn extract_asm(func: &ItemFn) -> Result<Punctuated<AsmOperand, Token![,]>> {
    if func.block.stmts.len() != 1 {
        bail!(
            func,
            "naked functions may only contain a single asm! statement"
        );
    }
    let (mac, attrs) = match &func.block.stmts[0] {
        Stmt::Macro(macro_) => (&macro_.mac, &macro_.attrs),
        Stmt::Expr(Expr::Macro(macro_), _) => (&macro_.mac, &macro_.attrs),
        _ => bail!(
            func,
            "naked functions may only contain a single asm! statement"
        ),
    };
    if !attrs.is_empty() || !mac.path.is_ident("asm") {
        bail!(
            func,
            "naked functions may only contain a single asm! statement"
        );
    }
    mac.parse_body_with(Punctuated::parse_terminated)
}
