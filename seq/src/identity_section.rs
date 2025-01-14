use crate::{
    ParseOutcome,
    PartialParser,
    ParseContext,
    ExpandContext,
    Expand,
};

#[derive(Clone)]
#[derive(Debug)]
pub struct IdentitySection {
    tokens: Vec<proc_macro2::TokenTree>
}

use quote::quote;

impl Expand for IdentitySection {
    fn expand(self, _ctx: &ExpandContext) -> proc_macro2::TokenStream {
        let ret_it = self.tokens.into_iter();
        quote! {
            #(#ret_it)*
        }
    }
}

impl PartialParser for IdentitySection {
    type Output = Self;
    fn parse(
        ctx: &ParseContext,
        input: syn::parse::ParseStream
    ) -> syn::parse::Result<ParseOutcome<Self::Output>>
    {
        let stop_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || input.peek2(syn::Token![~]) 
            || input.peek(syn::Token![#]) 
            || fork.parse::<proc_macro2::Group>().is_ok() {
                return true;
            }

            if let Ok(ident) = fork.parse::<proc_macro2::Ident>() {
               ident == ctx.iter_ident
            } else {
                false
            }
        };

        eprintln!("parse_identity: pre-visit: {:?}", input);
        if stop_cond(input) {
            // try parsing another way if possible
            return Ok(ParseOutcome::RecoverableError);
        }

        let mut ret = IdentitySection {
            tokens: vec![],
        };

        while !stop_cond(input) {
            match input.parse::<proc_macro2::TokenTree>() {
                Ok(tt) => ret.tokens.push(tt),
                Err(err) => return Err(err),
            }
        }
        eprintln!("parse_identity: post-visit: {:?}", input);
        Ok(ParseOutcome::Valid(ret))

    }
}

