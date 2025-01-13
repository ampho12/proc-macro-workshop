use crate::{
    ParseOutcome,
    PartialParser,
    ParseContext,
    SectionTree,
    try_extract_section_tree
};


use crate::concat_section::ConcatSection;
use crate::identity_section::IdentitySection;
use crate::replace_section::ReplaceSection;
use crate::no_nest_repeat_group::NoNestRepeatGroup;

use syn::parse::Parser;

#[derive(Debug)]
pub struct RepeatGroup {
    sections: Vec<SectionTree>
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
        let _ = input.parse::<syn::Token![#]>();

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return Ok(ParseOutcome::RecoverableError);
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<RepeatGroup> {

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

            Ok(ret)
        };

        let ret = parser.parse2(g.stream());
        eprintln!("parse_repeat_group: post-visit: {:?}", input);

        match ret {
            Ok(repeat_group) => Ok(ParseOutcome::Valid(repeat_group)),
            Err(error) => Err(error),
        }
    }
}
