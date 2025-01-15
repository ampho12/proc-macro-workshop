use crate::{
    ParseOutcome,
    PartialParser,
    ParseContext,
    SectionTree,
    ExpandContext,
    Expand,
    try_extract_section_tree
};

use quote::quote;

use crate::identity_section::IdentitySection;
use crate::repeat_group::RepeatGroup;

use syn::parse::Parser;

#[derive(Clone)]
#[derive(Debug)]
pub struct IdentityGroup {
    sections: Vec<SectionTree>,
    delimiter: proc_macro2::Delimiter,
    span: proc_macro2::Span,
}

impl Expand for IdentityGroup {
    fn expand(self, ctx: &ExpandContext) -> proc_macro2::TokenStream {
        let ret_it = self.sections.into_iter().flat_map(|sect| {
            sect.expand(ctx).into_iter()
        });

        let mut ret = proc_macro2::TokenTree::Group(proc_macro2::Group::new(
            self.delimiter,
            quote! {
                #(#ret_it)*
            }
        ));
        ret.set_span(self.span);
        let ret = quote! { #ret };
        eprintln!("returning : {:?}", ret);
        ret
    }
}

impl PartialParser for IdentityGroup {
    type Output = Self;
    fn parse(
        ctx: &ParseContext,
        input: syn::parse::ParseStream
    ) -> syn::parse::Result<ParseOutcome<Self::Output>>
    {
        let start_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || fork.parse::<proc_macro2::Group>().is_err() {
                return false;
            }
            true
        };

        if !start_cond(input) {
            // try parsing another way if possible
            return Ok(ParseOutcome::RecoverableError);
        }


        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<ParseOutcome<IdentityGroup>> {

            let mut ret = IdentityGroup {
                sections: vec![],
                delimiter: proc_macro2::Delimiter::None,
                span: proc_macro2::Span::call_site(),
            };

            let parsers = [
                try_extract_section_tree::<IdentitySection>,
                try_extract_section_tree::<RepeatGroup>,
                try_extract_section_tree::<IdentityGroup>,
            ];

            while !input.is_empty() {
                let mut parsed_smth = false;
                for parser in parsers {
                    match parser(ctx, input) {
                        Ok(ParseOutcome::Valid(sect)) => {
                            // eprintln!("parsed something");
                            ret.sections.push(sect);
                            parsed_smth = true;
                        }
                        Err(err) => return Err(err),
                        _ => {}
                    }
                }
                if !parsed_smth {
                    // cannot be parsed as idenitity group but can be as other groups
                    // must empty stream before returning
                    while !input.is_empty() {
                        let _ = input.parse::<proc_macro2::TokenTree>();
                    }
                    return Ok(ParseOutcome::RecoverableError);
                    // return Err(input.error("IdentityGroup: Unable to Parse"));
                }
            }

            Ok(ParseOutcome::Valid(ret))
        };

        // at this point, we have a group. We don't know if we can fully parse it
        // try to parse a forked stream
        let fork = input.fork();
        let Ok(g) = fork.parse::<proc_macro2::Group>() else {
            return Ok(ParseOutcome::RecoverableError);
        };

        eprintln!("here 1");
        let ret = parser.parse2(g.stream());
        eprintln!("ret: {:?}", ret);

        if let ParseOutcome::RecoverableError = ret? {
            eprintln!("got recoverable error!");
        }
        eprintln!("here 3");
        


        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return Ok(ParseOutcome::RecoverableError);
        };

        let ret = parser.parse2(g.stream());

        ret.map(|outcome| {
            match outcome {
                ParseOutcome::Valid(mut group) => {
                    group.delimiter = g.delimiter();
                    group.span = g.span();
                    ParseOutcome::Valid(group)
                }
                ParseOutcome::RecoverableError => ParseOutcome::RecoverableError,
            }
        })
    }
}
