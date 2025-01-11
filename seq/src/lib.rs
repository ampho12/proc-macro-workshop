use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;


enum RepeatSectionTree {
    Replace,
    Identity,
    Repeat,
    Concat,
    Group,
}


enum SectionTree {
    Replace,
    Identity,
    Repeat,
    Concat,
    Group,
}

struct ReplaceSection {
    tokens: Vec<proc_macro2::TokenTree>
}

#[derive(Debug)]
struct IdentitySection {
    tokens: Vec<proc_macro2::TokenTree>
}

#[derive(Debug)]
struct ConcatSection {
    tokens: Vec<proc_macro2::TokenTree>
}



struct SectionStream {
    sections: Vec<SectionTree>,
}


#[derive(Debug)]
struct ParseContext {
    iter_ident: proc_macro2::Ident,
    start: usize,
    end: usize,
}


impl syn::parse::Parse for ParseContext {
// impl ParseContext {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<ParseContext> {
        let iter_ident: syn::Ident = input.parse()?;
        input.parse::<syn::Token![in]>()?;
        let start_lit: syn::LitInt = input.parse()?;
        let start: usize;
        if let Ok(val) = start_lit.base10_parse::<usize>() {
            start = val;
        } else {
            return Err(input.error("Expected usize in base10"));
        }
        input.parse::<syn::Token![..]>()?;
        let end_lit: syn::LitInt = input.parse()?;
        let end: usize;
        if let Ok(val) = end_lit.base10_parse::<usize>() {
            end = val;
        } else {
            return Err(input.error("Expected usize in base10"));
        }

        // eprintln!("{:?} {:?} {:?}", iter_ident, start, end);
        // eprintln!("input: {:?}", input);
        Ok(ParseContext{
            iter_ident,
            start,
            end,
        })
    }
}



impl ParseContext {
    fn parse_identity(&self, input: syn::parse::ParseStream) -> syn::parse::Result<IdentitySection> {
        let stop_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || input.peek2(syn::Token![~]) 
            || fork.parse::<proc_macro2::Group>().is_ok() {
                return true;
            }

            if let Ok(ident) = fork.parse::<proc_macro2::Ident>() {
               ident == self.iter_ident
            } else {
                true
            }
        };

        let mut ret = IdentitySection {
            tokens: vec![],
        };

        eprintln!("parse_identity: pre-visit: {:?}", input);
        while !stop_cond(input) {
            ret.tokens.push(input.parse::<proc_macro2::TokenTree>()?);
        }
        eprintln!("parse_identity: post-visit: {:?}", input);

        if ret.tokens.is_empty() {
            return Err(input.error("Empty Identity Section"))
        }

        Ok(ret)
    }

    fn parse_concat(&self, input: syn::parse::ParseStream) -> syn::parse::Result<ConcatSection> {
        let start_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
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
            input.parse::<syn::Token![~]>()?;
            let fork = input.fork();
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
            return Err(input.error("Failed to meet start conditions for concat section"));
        }

        ret.tokens.push(input.parse::<proc_macro2::TokenTree>()?);
        while !stop_cond(input) {
            ret.tokens.push(parse_concat_link(input)?);
        }
        eprintln!("parse_concat: post-visit: {:?}", input);

        if ret.tokens.is_empty() {
            return Err(input.error("Empty Identity Section"))
        }

        Ok(ret)
    }
}



#[derive(Debug)]
struct SeqTree {
    parse_ctx: ParseContext,
}

impl syn::parse::Parse for SeqTree {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let parse_ctx: ParseContext = input.parse()?;

        let body;
        syn::braced!(body in input);

        let input = &body;
        // eprintln!("parse_ctx is {:?}", parse_ctx);

        // eprintln!("input is {:?}", input);
        let ident_sect = parse_ctx.parse_identity(input)?;
        eprintln!("ident_sect is {:?}\n", ident_sect);

        // eprintln!("input is {:?}", input);
        let ident_sect = parse_ctx.parse_concat(input)?;
        eprintln!("concat_sect is {:?}\n", ident_sect);

        while !input.is_empty() {
            input.parse::<proc_macro2::TokenTree>()?;
        }

        Ok(SeqTree{
            parse_ctx,
        })
    }
}



#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {

    // let input = input.parse::<syn::parse::ParseStream>()?;
    
    let seq_tree = syn::parse_macro_input!(input as SeqTree);
    
    // let parse_ctx = syn::parse::<ParseContext>(input);
    // eprintln!("{:?}", seq_tree);

    // let Ok(parse_ctx) = syn::parse::<ParseContext>(input) else {
    //     let err = syn::Error::new(proc_macro2::Span::call_site(), "Unable to parse header").to_compile_error();
    //     let ret = quote! {
    //         #err
    //     };
    //     return ret.into();
    // };
    // input.parse()?;
    // let header_parser = ParseContext::parse;
    // let parse_ctx = header_parser.parse(input);

    // let Ok(parse_ctx) = header_parser.parse(input) else {
    //     let err = syn::Error::new(proc_macro2::Span::call_site(), "Unable to parse header").to_compile_error();
    //     let ret = quote! {
    //         #err
    //     };
    //     return ret.into();
    // };



    // let parse_ctx: ParseContext = input.parse()?;

    
    TokenStream::new()
}
