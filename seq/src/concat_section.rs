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
pub struct ConcatSection {
    tokens: Vec<proc_macro2::TokenTree>
}

impl Expand for ConcatSection {
    fn expand(self, ctx: &ExpandContext) -> proc_macro2::TokenStream {

        let replaced = self.tokens.into_iter().map(|token| {
            match token {
                proc_macro2::TokenTree::Ident(ref ident) => {
                    if *ident == ctx.iter_ident {
                        let mut ret = ctx.target.clone();
                        ret.set_span(ident.span());
                        ret
                    } else {
                        token
                    }
                },
                _ => token,
            }
        });

        let concat:String = replaced.into_iter().flat_map(|tt| {
            // tt.to_string().chars().collect::<Vec<char>>()
            tt.to_string().chars().collect::<Vec<char>>().into_iter()
        }).collect();
        // eprintln!("concat: {:?}", concat);
        let ret = proc_macro2::TokenTree::Ident(
            proc_macro2::Ident::new(concat.as_str(), proc_macro2::Span::call_site())
        );
        quote! {
            #ret
        }
    }
}

impl PartialParser for ConcatSection {
    type Output = Self;
    fn parse(
        ctx: &ParseContext,
        input: syn::parse::ParseStream
    ) -> syn::parse::Result<ParseOutcome<Self::Output>>
    {
        let start_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || input.peek(syn::Token![#]) 
            || !input.peek2(syn::Token![~]) 
            || fork.parse::<proc_macro2::Group>().is_ok() {
                return false;
            }
            true
        };

        let stop_cond = |input: syn::parse::ParseStream| -> bool {
            if input.is_empty()
            || !input.peek(syn::Token![~]) {
                return true;
            }
            false
        };

        let parse_concat_link = |input: syn::parse::ParseStream| -> syn::parse::Result<proc_macro2::TokenTree> {

            let fork = input.fork();
            fork.parse::<syn::Token![~]>()?;
            if fork.is_empty() {
                return Err(input.error("Expected Token after '~'"));
            }
            input.parse::<syn::Token![~]>()?;

            if fork.peek(syn::Token![#]) {
                return Err(input.error("Cannot concatenate '#'"));
            }

            if fork.parse::<proc_macro2::Group>().is_ok() {
                return Err(input.error("Expected Non-Group Token"));
            }
            input.parse::<proc_macro2::TokenTree>()
        };

        let mut ret = ConcatSection {
            tokens: vec![],
        };

        eprintln!("parse_concat: pre-visit: {:?}", input);
        if !start_cond(input) {
            // try parsing another way if possible
            return Ok(ParseOutcome::RecoverableError);
        }

        match input.parse::<proc_macro2::TokenTree>() {
            Ok(tt) => ret.tokens.push(tt),
            Err(err) => return Err(err),
        }

        while !stop_cond(input) {
            match parse_concat_link(input) {
                Ok(tt) => ret.tokens.push(tt),
                Err(err) => return Err(err),
            }
        }
        eprintln!("parse_concat: post-visit: {:?}", input);
        Ok(ParseOutcome::Valid(ret))
    }
}
