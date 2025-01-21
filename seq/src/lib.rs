mod concat_section;
mod identity_section;
mod replace_section;
mod group;
mod repeat_group;

use concat_section::ConcatSection;
use identity_section::IdentitySection;
use replace_section::ReplaceSection;
use group::Group;
use repeat_group::RepeatGroup;


use proc_macro::TokenStream;
use quote::quote;

#[derive(Debug)]
enum ParseOutcome<T> {
    Valid(T),            // Fully valid result
    RecoverableError,    // Invalid but recoverable, e.g. parse another way
}

trait SectionVisitor<'ast> {
    type Output;
    fn visit_identity(&mut self, sect: &IdentitySection) -> Self::Output;
    fn visit_concat(&mut self, sect: &ConcatSection) -> Self::Output;
    fn visit_replace(&mut self, sect: &ReplaceSection) -> Self::Output;
    fn visit_repeat_group(&mut self, sect: &RepeatGroup) -> Self::Output;
    fn visit_group(&mut self, sect: &Group) -> Self::Output;
}

trait Visitee<'a> {
    fn visit<V: SectionVisitor<'a>>(&self, visitor: &mut V) -> <V as SectionVisitor<'a>>::Output;
}

impl<'a> Visitee<'a> for SectionTree {
    fn visit<V: SectionVisitor<'a>>(&self, visitor: &mut V) -> <V as SectionVisitor<'a>>::Output {
        match self {
            SectionTree::Identity(ref sect) => visitor.visit_identity(sect),
            SectionTree::Concat(ref sect) => visitor.visit_concat(sect),
            SectionTree::Replace(ref sect) => visitor.visit_replace(sect),
            SectionTree::Group(ref sect) => visitor.visit_group(sect),
            SectionTree::RepeatGroup(ref sect) => visitor.visit_repeat_group(sect),
        }
    }
}

struct CheckRepeatSection;
impl<'a> SectionVisitor<'a> for CheckRepeatSection {
    type Output = bool;

    fn visit_identity(&mut self, _sect: &IdentitySection) -> Self::Output {
        false
    }

    fn visit_concat(&mut self, _sect: &ConcatSection) -> Self::Output {
        false
    }

    fn visit_replace(&mut self, _sect: &ReplaceSection) -> Self::Output {
        false
    }

    fn visit_repeat_group(&mut self, _sect: &RepeatGroup) -> Self::Output {
        true
    }

    fn visit_group(&mut self, sect: &Group) -> Self::Output {
        let mut ret = false;
        for sect2 in sect.sections.iter() {
            ret |= sect2.visit(self);
        }
        ret
    }
}

struct SectionSanitizer {
    has_repeat_group: bool,
    in_repeat_group: bool,
}

impl SectionSanitizer {
    fn new(has_repeat_group: bool) -> Self {
        SectionSanitizer {
            has_repeat_group,
            in_repeat_group: false,
        }
    }
}

impl<'a> SectionVisitor<'a> for SectionSanitizer {
    type Output = bool;

    fn visit_identity(&mut self, _sect: &IdentitySection) -> Self::Output {
        true
    }

    fn visit_concat(&mut self, _sect: &ConcatSection) -> Self::Output {
        !self.has_repeat_group || self.in_repeat_group
    }

    fn visit_replace(&mut self, _sect: &ReplaceSection) -> Self::Output {
        !self.has_repeat_group || self.in_repeat_group
    }

    fn visit_repeat_group(&mut self, sect: &RepeatGroup) -> Self::Output {

        if self.in_repeat_group {
            return false;
        }

        self.in_repeat_group = true;

        for sect2 in sect.sections.iter() {
            if !sect2.visit(self) {
                return false;
            }
        }
        self.in_repeat_group = false;
        true
    }

    fn visit_group(&mut self, sect: &Group) -> Self::Output {
        for sect2 in sect.sections.iter() {
            if !sect2.visit(self) {
                return false
            }
        }
        true
    }
}





#[derive(Clone)]
#[derive(Debug)]
enum SectionTree {
    Identity(IdentitySection),
    Concat(ConcatSection),
    Replace(ReplaceSection),
    Group(Group),
    RepeatGroup(RepeatGroup),
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

impl From<RepeatGroup> for SectionTree {
    fn from(section: RepeatGroup) -> Self {
        SectionTree::RepeatGroup(section)
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
            SectionTree::Group(sect) => sect.expand(ctx),
            SectionTree::Concat(sect) => sect.expand(ctx),
            SectionTree::Replace(sect) => sect.expand(ctx),
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
    has_repeat_group: bool,
}

impl SeqTree {
    fn expand(self) -> proc_macro2::TokenStream {

        let ctx = ExpandContext {
            target: proc_macro2::TokenTree::Literal(
                proc_macro2::Literal::usize_unsuffixed(69)
            ),
            iter_ident: self.parse_ctx.iter_ident.clone(),
            start: self.parse_ctx.start,
            end: self.parse_ctx.end,
        };
        if self.has_repeat_group {
            let stream = self.base_section.expand(&ctx);
            let proc_macro2::TokenTree::Group(group) = stream.into_iter().next().unwrap() else {
                panic!("Didn't recieve group");
            };
            group.stream()
        } else {
            // we cannot simply repeat
            // iterate over the internal sections of the Group Section
            
            let SectionTree::Group(group) = self.base_section else {
                panic!("First section must be group section");
            };

            let ret_it = (ctx.start..ctx.end).flat_map(|n| {
                let mut expand_ctx: ExpandContext = ctx.clone();
                expand_ctx.target = proc_macro2::TokenTree::Literal(
                    proc_macro2::Literal::usize_unsuffixed(n)
                );
                group.sections.clone().into_iter().flat_map(move |sect| {
                    sect.expand(&expand_ctx).into_iter()
                })
            });

            quote! {
                #(#ret_it)*
            }
        }

    }
}


impl syn::parse::Parse for SeqTree {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let parse_ctx: ParseContext = input.parse()?;


        match try_extract::<Group>(&parse_ctx, input) {
            Ok(ParseOutcome::Valid(sect)) => {
                eprintln!("group: {:?}\n", sect);

                let has_repeat_group = SectionTree::from(sect.clone()).visit(&mut CheckRepeatSection {});
                let mut sanitizer = SectionSanitizer::new(has_repeat_group);
                // std::convert::Into::<SectionTree>::into(
                let ret = SectionTree::from(sect.clone()).visit(&mut sanitizer);
                if !ret {
                    panic!("Cannot sanitize");
                }
                return Ok(SeqTree{
                    parse_ctx,
                    base_section: sect.into(),
                    has_repeat_group,
                })
            }
            Ok(ParseOutcome::RecoverableError) => {},
            Err(err) => return Err(err),
        }

        // match try_extract::<BaseNestRepeatGroup>(&parse_ctx, input) {
        //     Ok(ParseOutcome::Valid(sect)) => {
        //         eprintln!("base_nest_repeat_group: {:?}\n", sect);
        //         return Ok(SeqTree{
        //             parse_ctx,
        //             base_section: sect.into(),
        //         })
        //     }
        //     Ok(ParseOutcome::RecoverableError) => {},
        //     Err(err) => return Err(err),
        // }

        // match try_extract::<BaseNoNestRepeatGroup>(&parse_ctx, input) {
        //     Ok(ParseOutcome::Valid(sect)) => {
        //         eprintln!("base_no_nest_repeat_group: {:?}\n", sect);
        //         return Ok(SeqTree{
        //             parse_ctx,
        //             base_section: sect.into(),
        //         })
        //     }
        //     Ok(ParseOutcome::RecoverableError) => {},
        //     Err(err) => return Err(err),
        // }

        Err(input.error("Unable to parse as anything"))
    }
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {

    // let input = input.parse::<syn::parse::ParseStream>()?;
    
    let seq_tree = syn::parse_macro_input!(input as SeqTree);

    let stream = seq_tree.expand();

    // let proc_macro2::TokenTree::Group(group) = stream.into_iter().next().unwrap() else {
    //     panic!("Didn't recieve group");
    // };

    // let ret = group.stream();

    eprintln!("ret: {:?}", stream);

    stream.into()
    
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
