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
use crate::concat_section::ConcatSection;
use crate::replace_section::ReplaceSection;
use crate::group::Group;

use syn::parse::Parser;


#[derive(Clone)]
#[derive(Debug)]
pub struct BaseNoNestRepeatGroup {
    sections: Vec<SectionTree>
}

impl Expand for BaseNoNestRepeatGroup {
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

impl PartialParser for BaseNoNestRepeatGroup {
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

        eprintln!("parse_base_no_nest_repeat_group: pre-visit: {:?}", input);
        if !start_cond(input) {
            // try parsing another way if possible
            return Ok(ParseOutcome::RecoverableError);
        }

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return Ok(ParseOutcome::RecoverableError);
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<BaseNoNestRepeatGroup> {

            let mut ret = BaseNoNestRepeatGroup {
                sections: vec![],
            };

            let parsers = [
                try_extract_section_tree::<IdentitySection>,
                try_extract_section_tree::<ConcatSection>,
                try_extract_section_tree::<ReplaceSection>,
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

        let ret = parser.parse2(g.stream());
        eprintln!("parse_base_no_nest_repeat_group: post-visit: {:?}", input);

        match ret {
            Ok(group) => Ok(ParseOutcome::Valid(group)),
            Err(_error) => Ok(ParseOutcome::RecoverableError),
        }
    }
}
