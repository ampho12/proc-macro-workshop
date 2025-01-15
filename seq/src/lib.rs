mod concat_section;
mod identity_section;
mod replace_section;
mod group;
mod identity_group;
mod no_nest_repeat_group;
mod repeat_group;
mod base_nest_repeat_group;
mod base_no_nest_repeat_group;

use concat_section::ConcatSection;
use identity_section::IdentitySection;
use replace_section::ReplaceSection;
use group::Group;
use identity_group::IdentityGroup;
use no_nest_repeat_group::NoNestRepeatGroup;
use repeat_group::RepeatGroup;
use base_nest_repeat_group::BaseNestRepeatGroup;
use base_no_nest_repeat_group::BaseNoNestRepeatGroup;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;

#[derive(Debug)]
enum ParseOutcome<T> {
    Valid(T),            // Fully valid result
    RecoverableError,    // Invalid but recoverable, e.g. parse another way
}

#[derive(Clone)]
#[derive(Debug)]
enum SectionTree {
    Identity(IdentitySection),
    Concat(ConcatSection),
    Replace(ReplaceSection),
    Group(Group),
    NoNestRepeatGroup(NoNestRepeatGroup),
    RepeatGroup(RepeatGroup),

    IdentityGroup(IdentityGroup),
    BaseNestRepeatGroup(BaseNestRepeatGroup),
    BaseNoNestRepeatGroup(BaseNoNestRepeatGroup),
}
impl From<IdentitySection> for SectionTree {
    fn from(section: IdentitySection) -> Self {
        SectionTree::Identity(section)
    }
}

impl From<ConcatSection> for SectionTree {
    fn from(section: ConcatSection) -> Self {
        SectionTree::Concat(section)
    }
}

impl From<ReplaceSection> for SectionTree {
    fn from(section: ReplaceSection) -> Self {
        SectionTree::Replace(section)
    }
}

impl From<Group> for SectionTree {
    fn from(section: Group) -> Self {
        SectionTree::Group(section)
    }
}

impl From<NoNestRepeatGroup> for SectionTree {
    fn from(section: NoNestRepeatGroup) -> Self {
        SectionTree::NoNestRepeatGroup(section)
    }
}

impl From<RepeatGroup> for SectionTree {
    fn from(section: RepeatGroup) -> Self {
        SectionTree::RepeatGroup(section)
    }
}

impl From<IdentityGroup> for SectionTree {
    fn from(section: IdentityGroup) -> Self {
        SectionTree::IdentityGroup(section)
    }
}

impl From<BaseNestRepeatGroup> for SectionTree {
    fn from(section: BaseNestRepeatGroup) -> Self {
        SectionTree::BaseNestRepeatGroup(section)
    }
}

impl From<BaseNoNestRepeatGroup> for SectionTree {
    fn from(section: BaseNoNestRepeatGroup) -> Self {
        SectionTree::BaseNoNestRepeatGroup(section)
    }
}

fn try_extract<P>(
    ctx: &ParseContext,
    input: syn::parse::ParseStream
) -> syn::parse::Result<ParseOutcome<P::Output>>
where
    P: PartialParser, <P as PartialParser>::Output: std::fmt::Debug,
{
    let fork = input.fork();
    if let ParseOutcome::RecoverableError = P::parse(ctx, &fork)? {
        return Ok(ParseOutcome::RecoverableError);
    }
    let ret = P::parse(ctx, input)?;

    if let ParseOutcome::Valid(ref outcome) = ret {
        eprintln!("parsed: {:?}", outcome);
    }
    Ok(ret)
}

fn try_extract_section_tree<T>(
    ctx: &ParseContext,
    input: syn::parse::ParseStream
) -> syn::parse::Result<ParseOutcome<SectionTree>>
where 
    T: PartialParser,
    T::Output: Into<SectionTree> + std::fmt::Debug,
{
    try_extract::<T>(ctx, input).map(|outcome| { 
        match outcome {
            ParseOutcome::Valid(sect) => ParseOutcome::Valid(sect.into()),
            ParseOutcome::RecoverableError => ParseOutcome::RecoverableError,
        }
    })
}

trait PartialParser {
    type Output;
    fn parse(
        ctx: &ParseContext,
        input: syn::parse::ParseStream
    ) -> syn::parse::Result<ParseOutcome<Self::Output>>;
}

trait Expand {
    fn expand(self, ctx: &ExpandContext) -> proc_macro2::TokenStream;
}


impl Expand for SectionTree {
    fn expand(self, ctx: &ExpandContext) -> proc_macro2::TokenStream {
        match self {
            SectionTree::Identity(sect) => sect.expand(ctx),
            SectionTree::IdentityGroup(sect) => sect.expand(ctx),
            SectionTree::Group(sect) => sect.expand(ctx),
            SectionTree::Concat(sect) => sect.expand(ctx),
            SectionTree::Replace(sect) => sect.expand(ctx),
            SectionTree::BaseNoNestRepeatGroup(sect) => sect.expand(ctx),
            SectionTree::NoNestRepeatGroup(sect) => sect.expand(ctx),
            SectionTree::BaseNestRepeatGroup(sect) => sect.expand(ctx),
            SectionTree::RepeatGroup(sect) => sect.expand(ctx),
        }
    }

}

#[derive(Clone)]
#[derive(Debug)]
struct ExpandContext {
    target: proc_macro2::TokenTree,
    iter_ident: proc_macro2::Ident,
    start: usize,
    end: usize,
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

        let mut is_inclusive = false;
        if input.peek(syn::token::Eq) {
            input.parse::<syn::Token![=]>()?;
            is_inclusive = true;
        }

        let end_lit: syn::LitInt = input.parse()?;
        let mut end: usize;
        if let Ok(val) = end_lit.base10_parse::<usize>() {
            end = val;
        } else {
            return Err(input.error("Expected usize in base10"));
        }

        if is_inclusive {
            end += 1;
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

#[derive(Debug)]
struct SeqTree {
    parse_ctx: ParseContext,
    base_section: SectionTree,
}

impl SeqTree {
    fn expand(self) -> proc_macro2::TokenStream {

        let expand_context = ExpandContext {
            target: proc_macro2::TokenTree::Literal(
                proc_macro2::Literal::usize_unsuffixed(69)
            ),
            iter_ident: self.parse_ctx.iter_ident.clone(),
            start: self.parse_ctx.start,
            end: self.parse_ctx.end,
        };

        self.base_section.expand(&expand_context)
    }
}


impl syn::parse::Parse for SeqTree {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let parse_ctx: ParseContext = input.parse()?;
        // eprintln!("parse_ctx is {:?}", parse_ctx);


        match try_extract::<BaseNestRepeatGroup>(&parse_ctx, input) {
            Ok(ParseOutcome::Valid(sect)) => {
                eprintln!("base_nest_repeat_group: {:?}\n", sect);
                return Ok(SeqTree{
                    parse_ctx,
                    base_section: sect.into(),
                })
            }
            Ok(ParseOutcome::RecoverableError) => {},
            Err(err) => return Err(err),
        }

        match try_extract::<BaseNoNestRepeatGroup>(&parse_ctx, input) {
            Ok(ParseOutcome::Valid(sect)) => {
                eprintln!("base_no_nest_repeat_group: {:?}\n", sect);
                return Ok(SeqTree{
                    parse_ctx,
                    base_section: sect.into(),
                })
            }
            Ok(ParseOutcome::RecoverableError) => {},
            Err(err) => return Err(err),
        }

        Err(input.error("Unable to parse as anything"))
    }
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {

    // let input = input.parse::<syn::parse::ParseStream>()?;
    
    let seq_tree = syn::parse_macro_input!(input as SeqTree);

    seq_tree.expand().into()
    
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
    
    // TokenStream::new()
}
