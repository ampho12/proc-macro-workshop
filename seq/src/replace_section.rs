use crate::{
    ParseOutcome,
    PartialParser,
    ParseContext,
    ExpandContext,
    Expand,
};

use quote::quote;

#[derive(Clone)]
#[derive(Debug)]
pub struct ReplaceSection;

impl Expand for ReplaceSection {
    fn expand(self, ctx: &ExpandContext) -> proc_macro2::TokenStream {
        // let ret_it = vec![ctx.target.clone()].into_iter();
        let tt = ctx.target.clone();
        quote! {
            #tt
            // #(#ret_it)*
        }
    }
}


impl PartialParser for ReplaceSection {
    type Output = Self;
    fn parse(
        ctx: &ParseContext,
        input: syn::parse::ParseStream
    ) -> syn::parse::Result<ParseOutcome<Self::Output>>
    {
        let stop_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || input.peek(syn::Token![#]) 
            || input.peek2(syn::Token![~]) {
                return true;
            }

            if let Ok(ident) = fork.parse::<proc_macro2::Ident>() {
                ident != ctx.iter_ident
            } else {
                true
            }
        };

        eprintln!("parse_replace: pre-visit: {:?}", input);
        if stop_cond(input) {
            Ok(ParseOutcome::RecoverableError)
        } else {
            match input.parse::<proc_macro2::TokenTree>() {
                Ok(_tt) => Ok(ParseOutcome::Valid(ReplaceSection {})),
                Err(err) => Err(err),
            }
        }

    }
}
