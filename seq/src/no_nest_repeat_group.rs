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

use syn::parse::Parser;

#[derive(Clone)]
#[derive(Debug)]
pub struct NoNestRepeatGroup {
    sections: Vec<SectionTree>
}

impl Expand for NoNestRepeatGroup {
    fn expand(self, ctx: &ExpandContext) -> proc_macro2::TokenStream {
        let ret_it = self.sections.into_iter().flat_map(|sect| {
            sect.expand(ctx).into_iter()
        });

        quote! {
            #(#ret_it)*
        }
    }
}

impl PartialParser for NoNestRepeatGroup {
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

        eprintln!("parse_no_nest_repeat_group: pre-visit: {:?}", input);
        if !start_cond(input) {
            // try parsing another way if possible
            return Ok(ParseOutcome::RecoverableError);
        }

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return Ok(ParseOutcome::RecoverableError);
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<NoNestRepeatGroup> {

            let mut ret = NoNestRepeatGroup {
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
                    return Err(input.error("NoNestRepeatGroup: Unable to Parse"));
                }
            }

            Ok(ret)
        };

        let ret = parser.parse2(g.stream());
        eprintln!("parse_group: post-visit: {:?}", input);

        match ret {
            Ok(no_nest_repeat_group) => Ok(ParseOutcome::Valid(no_nest_repeat_group)),
            Err(error) => Err(error),
        }
    }
}
