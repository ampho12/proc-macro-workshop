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

use crate::identity_group::IdentityGroup;
use crate::identity_section::IdentitySection;
use crate::repeat_group::RepeatGroup;

use syn::parse::Parser;

#[derive(Clone)]
#[derive(Debug)]
pub struct BaseNestRepeatGroup {
    sections: Vec<SectionTree>
}

impl Expand for BaseNestRepeatGroup {
    fn expand(self, ctx: &ExpandContext) -> proc_macro2::TokenStream {
        let ret_it = self.sections.into_iter().flat_map(|sect| {
            sect.expand(ctx).into_iter()
        });

        quote! {
            #(#ret_it)*
        }
    }
}

impl PartialParser for BaseNestRepeatGroup {
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

        eprintln!("parse_base_nest_repeat_group: pre-visit: {:?}", input);
        if !start_cond(input) {
            // try parsing another way if possible
            return Ok(ParseOutcome::RecoverableError);
        }

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return Ok(ParseOutcome::RecoverableError);
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<ParseOutcome<BaseNestRepeatGroup>> {

            let mut ret = BaseNestRepeatGroup {
                sections: vec![],
            };

            let parsers = [
                try_extract_section_tree::<IdentitySection>,
                try_extract_section_tree::<IdentityGroup>,
                try_extract_section_tree::<RepeatGroup>,
            ];

            let mut parsed_identity_group = false;
            while !input.is_empty() {
                let mut parsed_smth = false;
                for parser in parsers {
                    match parser(ctx, input) {
                        Ok(ParseOutcome::Valid(sect)) => {
                            if let SectionTree::IdentityGroup(ref _a) = sect {
                                parsed_identity_group = true;
                            }
                            // eprintln!("parsed something");
                            ret.sections.push(sect);
                            parsed_smth = true;
                        }
                        Err(err) => {
                            return Err(err);
                        }
                        _ => {}
                    }
                }
                if !parsed_smth {
                    // return Err(input.error("Unable to Parse"));
                    // cannot parse as this type but recovery possible
                    // e.g. parse as another type
                    while !input.is_empty() {
                        let _ = input.parse::<proc_macro2::TokenTree>();
                    }
                    return Ok(ParseOutcome::RecoverableError);
                }

            }
            if !parsed_identity_group {
                return Ok(ParseOutcome::RecoverableError);
            }

            Ok(ParseOutcome::Valid(ret))
        };

        let ret = parser.parse2(g.stream());
        eprintln!("parse_base_nest_repeat_group: post-visit: {:?}", input);
        ret
    }
}
