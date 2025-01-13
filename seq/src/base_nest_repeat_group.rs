use crate::{
    ParseOutcome,
    PartialParser,
    ParseContext,
    SectionTree,
    try_extract_section_tree
};


use crate::identity_group::IdentityGroup;
use crate::identity_section::IdentitySection;
use crate::repeat_group::RepeatGroup;

use syn::parse::Parser;

#[derive(Debug)]
pub struct BaseNestRepeatGroup {
    sections: Vec<SectionTree>
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

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<BaseNestRepeatGroup> {

            let mut ret = BaseNestRepeatGroup {
                sections: vec![],
            };

            let parsers = [
                try_extract_section_tree::<IdentitySection>,
                try_extract_section_tree::<IdentityGroup>,
                try_extract_section_tree::<RepeatGroup>,
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
        eprintln!("parse_base_nest_repeat_group: post-visit: {:?}", input);

        match ret {
            Ok(repeat_group) => Ok(ParseOutcome::Valid(repeat_group)),
            Err(_error) => Ok(ParseOutcome::RecoverableError),
        }
    }
}
