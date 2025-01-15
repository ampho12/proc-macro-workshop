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

use crate::concat_section::ConcatSection;
use crate::identity_section::IdentitySection;
use crate::replace_section::ReplaceSection;
use crate::no_nest_repeat_group::NoNestRepeatGroup;

use syn::parse::Parser;

#[derive(Clone)]
#[derive(Debug)]
pub struct RepeatGroup {
    sections: Vec<SectionTree>
}

impl Expand for RepeatGroup {
    fn expand(self, ctx: &ExpandContext) -> proc_macro2::TokenStream {

        let ret_it = (ctx.start..ctx.end).flat_map(|n| {
            let mut expand_ctx: ExpandContext = ctx.clone();
            expand_ctx.target = proc_macro2::TokenTree::Literal(
                proc_macro2::Literal::usize_unsuffixed(n)
            );
            self.sections.clone().into_iter().flat_map(move |sect| {
                sect.expand(&expand_ctx).into_iter()
            })
        });

        quote! {
            #(#ret_it)*
        }
    }
}

impl PartialParser for RepeatGroup {
    type Output = Self;
    fn parse(
        ctx: &ParseContext,
        input: syn::parse::ParseStream
    ) -> syn::parse::Result<ParseOutcome<Self::Output>>
    {
        let start_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || !input.peek(syn::Token![#]) {
                return false;
            }

            // parse the hashtag
            let _ = fork.parse::<syn::Token![#]>();
            if fork.is_empty() {
                return false;
            }

            if let Ok(group) = fork.parse::<proc_macro2::Group>() {
                group.delimiter() == proc_macro2::Delimiter::Parenthesis
            } else {
                false
            }
        };

        eprintln!("parse_repeat_group: pre-visit: {:?}", input);
        if !start_cond(input) {
            // try parsing another way if possible
            return Ok(ParseOutcome::RecoverableError);
        }
        input.parse::<syn::Token![#]>()?;

        let fork = input.fork();
        fork.parse::<proc_macro2::Group>()?;

        if fork.is_empty() || !fork.peek(syn::Token![*]) {
            return Err(input.error("Expected '*'"));
        }

        let g = input.parse::<proc_macro2::Group>()?;
        let _ = input.parse::<syn::Token![*]>()?;

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<ParseOutcome<RepeatGroup>> {

            let mut ret = RepeatGroup {
                sections: vec![],
            };

            let parsers = [
                try_extract_section_tree::<IdentitySection>,
                try_extract_section_tree::<ConcatSection>,
                try_extract_section_tree::<ReplaceSection>,
                try_extract_section_tree::<NoNestRepeatGroup>,
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
                    return Err(input.error("RepeatGroup: Unable to Parse"));
                }
            }

            Ok(ParseOutcome::Valid(ret))
        };

        let ret = parser.parse2(g.stream());
        eprintln!("parse_repeat_group: post-visit: {:?}", input);
        ret

        // match ret {
        //     Ok(repeat_group) => Ok(ParseOutcome::Valid(repeat_group)),
        //     Err(error) => Err(error),
        // }
    }
}
