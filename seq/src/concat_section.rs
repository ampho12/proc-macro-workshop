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
    tokens: Vec<proc_macro2::TokenTree>,
    src_span: proc_macro2::Span,
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
        let mut ret = proc_macro2::TokenTree::Ident(
            proc_macro2::Ident::new(concat.as_str(), proc_macro2::Span::call_site())
        );
        ret.set_span(self.src_span);
        
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
            // this has to be tilde
            let tilde = input.parse::<proc_macro2::TokenTree>()?;

            if fork.peek(syn::Token![#]) {
                return Err(input.error("Cannot concatenate '#'"));
            }

            if fork.parse::<proc_macro2::Group>().is_ok() {
                return Err(input.error("Expected Non-Group Token"));
            }
            let mut ret = input.parse::<proc_macro2::TokenTree>()?;

            // let span1 = proc_macro2::Span::call_site();
            // let span2 = proc_macro2::Span::call_site();
            // eprintln!("first: {:?}", span1);
            // eprintln!("second: {:?}", span2);
            // eprintln!("join1: {:?}", span1.join(span2));

            // eprintln!("first: {:?}", tilde.span());
            // eprintln!("second: {:?}", ret.span());
            // eprintln!("join1: {:?}", ret.span().join(tilde.span()));


            // ret.set_span(tilde.span().join(ret.span()).unwrap());
            Ok(ret)
        };

        let mut ret = ConcatSection {
            tokens: vec![],
            src_span: proc_macro2::Span::call_site(),
        };

        // eprintln!("parse_concat: pre-visit: {:?}", input);
        if !start_cond(input) {
            // try parsing another way if possible
            return Ok(ParseOutcome::RecoverableError);
        }
        let mut src_span;

        match input.parse::<proc_macro2::TokenTree>() {
            Ok(tt) => {
                src_span = tt.span();
                ret.tokens.push(tt);
            }
            Err(err) => return Err(err),
        }

        while !stop_cond(input) {
            match parse_concat_link(input) {
                Ok(tt) => {
                    // src_span = proc_macro2::Span::join(&src_span, tt.span()).unwrap();
                    ret.tokens.push(tt);
                }
                Err(err) => return Err(err),
            }
        }
        ret.src_span = src_span;
        // eprintln!("parse_concat: post-visit: {:?}", input);
        Ok(ParseOutcome::Valid(ret))
    }
}
