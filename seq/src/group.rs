use crate::{
    ParseOutcome,
    PartialParser,
    ParseContext,
    ExpandContext,
    Expand,
    SectionTree,
    try_extract_section_tree
};

use quote::quote;

use crate::concat_section::ConcatSection;
use crate::identity_section::IdentitySection;
use crate::replace_section::ReplaceSection;
use crate::repeat_group::RepeatGroup;

use syn::parse::Parser;

#[derive(Clone)]
#[derive(Debug)]
pub struct Group {
    pub sections: Vec<SectionTree>,
    delimiter: proc_macro2::Delimiter,
    span: proc_macro2::Span,
}

impl Expand for Group {
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
        quote! { #ret }
    }
}

impl PartialParser for Group {
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

        eprintln!("parse_group: pre-visit: {:?}", input);
        if !start_cond(input) {
            // try parsing another way if possible
            return Ok(ParseOutcome::RecoverableError);
        }

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return Ok(ParseOutcome::RecoverableError);
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<Group> {

            let mut ret = Group {
                sections: vec![],
                delimiter: proc_macro2::Delimiter::None,
                span: proc_macro2::Span::call_site(),
            };

            let parsers = [
                try_extract_section_tree::<IdentitySection>,
                try_extract_section_tree::<ConcatSection>,
                try_extract_section_tree::<ReplaceSection>,
                try_extract_section_tree::<RepeatGroup>,
                try_extract_section_tree::<Group>,
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
                    return Err(input.error("Unable to Parse"));
                }
            }

            Ok(ret)
        };

        let token_stream = g.stream();
        let ret = parser.parse2(token_stream);
        eprintln!("parse_group: post-visit: {:?}", input);
        match ret {
            Ok(mut group) => {
                group.delimiter = g.delimiter();
                group.span = g.span();
                Ok(ParseOutcome::Valid(group))
            }
            Err(error) => Err(error),
        }
    }
}
