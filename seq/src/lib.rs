use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;

enum ParseOutcome<T> {
    Valid(T),            // Fully valid result
    RecoverableError,    // Invalid but recoverable, e.g. parse another way
    FatalError(syn::Error), // Unrecoverable error
}

#[derive(Debug)]
enum SectionTree {
    Identity(IdentitySection),
    Concat(ConcatSection),
    Group(Group),
}

#[derive(Debug)]
struct IdentitySection {
    tokens: Vec<proc_macro2::TokenTree>
}

#[derive(Debug)]
struct ConcatSection {
    tokens: Vec<proc_macro2::TokenTree>
}

#[derive(Debug)]
struct Group {
    sections: Vec<SectionTree>
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
    fn parse_identity(&self, input: syn::parse::ParseStream) -> ParseOutcome<IdentitySection> {
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

        eprintln!("parse_identity: pre-visit: {:?}", input);
        if stop_cond(input) {
            // try parsing another way if possible
            return ParseOutcome::RecoverableError;
        }

        let mut ret = IdentitySection {
            tokens: vec![],
        };

        while !stop_cond(input) {
            match input.parse::<proc_macro2::TokenTree>() {
                Ok(tt) => ret.tokens.push(tt),
                Err(err) => return ParseOutcome::FatalError(err),
            }
        }
        eprintln!("parse_identity: post-visit: {:?}", input);
        ParseOutcome::Valid(ret)
    }

    fn parse_concat(&self, input: syn::parse::ParseStream) -> ParseOutcome<ConcatSection> {
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

            let fork = input.fork();
            fork.parse::<syn::Token![~]>()?;
            if fork.is_empty() {
                return Err(input.error("Expected Token after '~'"));
            }

            input.parse::<syn::Token![~]>()?;
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
            return ParseOutcome::RecoverableError;
        }

        match input.parse::<proc_macro2::TokenTree>() {
            Ok(tt) => ret.tokens.push(tt),
            Err(err) => return ParseOutcome::FatalError(err),
        }
        while !stop_cond(input) {
            match parse_concat_link(input) {
                Ok(tt) => ret.tokens.push(tt),
                Err(err) => return ParseOutcome::FatalError(err),
            }
        }
        eprintln!("parse_concat: post-visit: {:?}", input);
        ParseOutcome::Valid(ret)
    }

    fn parse_group(&self, input: syn::parse::ParseStream) -> ParseOutcome<Group> {

        let start_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || fork.parse::<proc_macro2::Group>().is_err() {
                return false;
            }
            true
        };

        eprintln!("parse_group: pre-visit: {:?}", input);
        if !start_cond(input) {
            // try parsing another way if possible
            return ParseOutcome::RecoverableError;
        }

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return ParseOutcome::RecoverableError;
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<Group> {

            let mut ret = Group {
                sections: vec![],
            };

            while !input.is_empty() {

                match self.parse_identity(input) {
                    ParseOutcome::Valid(sect) => {
                        ret.sections.push(SectionTree::Identity(sect));
                        continue;
                    }
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }

                match self.parse_concat(input) {
                    ParseOutcome::Valid(sect) => {
                        ret.sections.push(SectionTree::Concat(sect));
                        continue;
                    }
                    // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Concat(sect)),
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }

                match self.parse_group(input) {
                    ParseOutcome::Valid(sect) => {
                        ret.sections.push(SectionTree::Group(sect));
                        continue;
                    }
                    // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Group(sect)),
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }
                
                // No branch matched, Unrecoverable
                return Err(input.error("Unable to Parse"));
            }

            Ok(ret)
        };

        let ret = parser.parse2(g.stream());
        eprintln!("parse_group: post-visit: {:?}", input);

        match ret {
            Ok(group) => ParseOutcome::Valid(group),
            Err(error) => ParseOutcome::FatalError(error),
        }
    }
}

#[derive(Debug)]
struct SeqTree {
    parse_ctx: ParseContext,
}

impl syn::parse::Parse for SeqTree {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let parse_ctx: ParseContext = input.parse()?;
        // eprintln!("parse_ctx is {:?}", parse_ctx);

        let body;
        syn::braced!(body in input);
        let input = &body;
        
        match parse_ctx.parse_identity(input) {
            ParseOutcome::Valid(ident_sect) => eprintln!("ident_sect: {:?}\n", ident_sect),
            ParseOutcome::RecoverableError => eprintln!("Cannot parse as Identity Section"),
            ParseOutcome::FatalError(err) => return Err(err),
        }
        
        match parse_ctx.parse_concat(input) {
            ParseOutcome::Valid(concat_sect) => eprintln!("concat_sect: {:?}\n", concat_sect),
            ParseOutcome::RecoverableError => eprintln!("Cannot parse as Concat Section"),
            ParseOutcome::FatalError(err) => return Err(err),
        }

        match parse_ctx.parse_group(input) {
            ParseOutcome::Valid(group) => eprintln!("group: {:?}\n", group),
            ParseOutcome::RecoverableError => eprintln!("Cannot parse as Group"),
            ParseOutcome::FatalError(err) => return Err(err),
        }

        // let group = parse_ctx.parse_group(input)?;
        // eprintln!("group is {:?}\n", group);

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
