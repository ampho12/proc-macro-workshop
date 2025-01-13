/* 
* Refactoring TODOs:
* 1. parser trait functions can return only a syn::parser::Error<T>. If we want to return
*    recoverable errors from this type, we are limited. Thus, instead of returning parsed object,
*    return something like
*
*    ```
*    enum ParseOutcome<T> {
*       Success(T),
*       RecoverableError,
*    }
*    ```
*
*    and then return `syn::parse::Result<ParseOutcome<T>>` from the syn parse functions. Use the
*    `syn::Error` as a fatal error.
*
* 2. Boilerplate reduction: A lot of code is boilerplate that is duplicated for error propagation.
*    This can be reduced by provied helper functions. Impelemnt the `try_parse` function
*
* 3. Parser iterators: iterate over possible parsers for the parsestream at any stage
*/

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;

enum ParseOutcome<T> {
    Valid(T),            // Fully valid result
    RecoverableError,    // Invalid but recoverable, e.g. parse another way
    FatalError(syn::Error), // Unrecoverable error
}

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

enum ParseOutcome2<T> {
    Valid(T),            // Fully valid result
    RecoverableError,    // Invalid but recoverable, e.g. parse another way
}


fn try_extract<P>(
    ctx: &ParseContext,
    input: syn::parse::ParseStream
) -> syn::parse::Result<ParseOutcome<P::Output>>
where
    P: PartialParser
{
    let fork = input.fork();
    P::parse(ctx, &fork)?;
    P::parse(ctx, input)
}


trait PartialParser {
    type Output;
    fn parse(
        ctx: &ParseContext,
        input: syn::parse::ParseStream
    ) -> syn::parse::Result<ParseOutcome<Self::Output>>;
}


#[derive(Debug)]
struct ReplaceSection;

impl PartialParser for ReplaceSection {
    type Output = Self;
    fn parse(
        ctx: &ParseContext,
        input: syn::parse::ParseStream
    ) -> syn::parse::Result<ParseOutcome<Self::Output>>
    {
        let stop_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || input.peek(syn::Token![#]) 
            || input.peek2(syn::Token![~]) {
                return true;
            }

            if let Ok(ident) = fork.parse::<proc_macro2::Ident>() {
                ident != ctx.iter_ident
            } else {
                true
            }
        };

        eprintln!("parse_replace: pre-visit: {:?}", input);
        if stop_cond(input) {
            Ok(ParseOutcome::RecoverableError)
        } else {
            match input.parse::<proc_macro2::TokenTree>() {
                Ok(_tt) => Ok(ParseOutcome::Valid(ReplaceSection {})),
                Err(err) => Err(err),
            }
        }

    }
}




#[derive(Debug)]
struct IdentitySection {
    tokens: Vec<proc_macro2::TokenTree>
}

#[derive(Debug)]
struct ConcatSection {
    tokens: Vec<proc_macro2::TokenTree>
}
impl PartialParser for ConcatSection {
    type Output = Self;
    fn parse(
        ctx: &ParseContext,
        input: syn::parse::ParseStream
    ) -> syn::parse::Result<ParseOutcome<Self::Output>>
    {
        let start_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || input.peek(syn::Token![#]) 
            || !input.peek2(syn::Token![~]) 
            || fork.parse::<proc_macro2::Group>().is_ok() {
                return false;
            }
            true
        };

        let stop_cond = |input: syn::parse::ParseStream| -> bool {
            if input.is_empty()
            || !input.peek(syn::Token![~]) {
                return true;
            }
            false
        };

        let parse_concat_link = |input: syn::parse::ParseStream| -> syn::parse::Result<proc_macro2::TokenTree> {

            let fork = input.fork();
            fork.parse::<syn::Token![~]>()?;
            if fork.is_empty() {
                return Err(input.error("Expected Token after '~'"));
            }
            input.parse::<syn::Token![~]>()?;

            if fork.peek(syn::Token![#]) {
                return Err(input.error("Cannot concatenate '#'"));
            }

            if fork.parse::<proc_macro2::Group>().is_ok() {
                return Err(input.error("Expected Non-Group Token"));
            }
            input.parse::<proc_macro2::TokenTree>()
        };

        let mut ret = ConcatSection {
            tokens: vec![],
        };

        eprintln!("parse_concat: pre-visit: {:?}", input);
        if !start_cond(input) {
            // try parsing another way if possible
            return Ok(ParseOutcome::RecoverableError);
        }

        match input.parse::<proc_macro2::TokenTree>() {
            Ok(tt) => ret.tokens.push(tt),
            Err(err) => return Err(err),
        }

        while !stop_cond(input) {
            match parse_concat_link(input) {
                Ok(tt) => ret.tokens.push(tt),
                Err(err) => return Err(err),
            }
        }
        eprintln!("parse_concat: post-visit: {:?}", input);
        Ok(ParseOutcome::Valid(ret))
    }
}

#[derive(Debug)]
struct Group {
    sections: Vec<SectionTree>
}

#[derive(Debug)]
struct IdentityGroup {
    sections: Vec<SectionTree>
}

#[derive(Debug)]
struct NoNestRepeatGroup {
    sections: Vec<SectionTree>
}

#[derive(Debug)]
struct RepeatGroup {
    sections: Vec<SectionTree>
}

#[derive(Debug)]
struct BaseNestRepeatGroup {
    sections: Vec<SectionTree>
}

#[derive(Debug)]
struct BaseNoNestRepeatGroup {
    sections: Vec<SectionTree>
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
        let end_lit: syn::LitInt = input.parse()?;
        let end: usize;
        if let Ok(val) = end_lit.base10_parse::<usize>() {
            end = val;
        } else {
            return Err(input.error("Expected usize in base10"));
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

impl ParseContext {
    fn parse_identity(&self, input: syn::parse::ParseStream) -> ParseOutcome<IdentitySection> {
        let stop_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || input.peek2(syn::Token![~]) 
            || input.peek(syn::Token![#]) 
            || fork.parse::<proc_macro2::Group>().is_ok() {
                return true;
            }

            if let Ok(ident) = fork.parse::<proc_macro2::Ident>() {
               ident == self.iter_ident
            } else {
                false
            }
        };

        eprintln!("parse_identity: pre-visit: {:?}", input);
        if stop_cond(input) {
            // try parsing another way if possible
            return ParseOutcome::RecoverableError;
        }

        let mut ret = IdentitySection {
            tokens: vec![],
        };

        while !stop_cond(input) {
            match input.parse::<proc_macro2::TokenTree>() {
                Ok(tt) => ret.tokens.push(tt),
                Err(err) => return ParseOutcome::FatalError(err),
            }
        }
        eprintln!("parse_identity: post-visit: {:?}", input);
        ParseOutcome::Valid(ret)
    }

    fn parse_replace(&self, input: syn::parse::ParseStream) -> ParseOutcome<ReplaceSection> {
        let stop_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || input.peek(syn::Token![#]) 
            || input.peek2(syn::Token![~]) {
                return true;
            }

            if let Ok(ident) = fork.parse::<proc_macro2::Ident>() {
               ident != self.iter_ident
            } else {
                true
            }
        };

        eprintln!("parse_replace: pre-visit: {:?}", input);
        if stop_cond(input) {
            // try parsing another way if possible
            return ParseOutcome::RecoverableError;
        } else {
            match input.parse::<proc_macro2::TokenTree>() {
                Ok(_tt) => {},
                Err(err) => return ParseOutcome::FatalError(err),
            }
        }

        eprintln!("parse_replace: post-visit: {:?}", input);
        ParseOutcome::Valid(ReplaceSection)
    }

    fn parse_concat(&self, input: syn::parse::ParseStream) -> ParseOutcome<ConcatSection> {
        let start_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || input.peek(syn::Token![#]) 
            || !input.peek2(syn::Token![~]) 
            || fork.parse::<proc_macro2::Group>().is_ok() {
                return false;
            }
            true
        };

        let stop_cond = |input: syn::parse::ParseStream| -> bool {
            if input.is_empty()
            || !input.peek(syn::Token![~]) {
                return true;
            }
            false
        };

        let parse_concat_link = |input: syn::parse::ParseStream| -> syn::parse::Result<proc_macro2::TokenTree> {

            let fork = input.fork();
            fork.parse::<syn::Token![~]>()?;
            if fork.is_empty() {
                return Err(input.error("Expected Token after '~'"));
            }
            input.parse::<syn::Token![~]>()?;

            if fork.peek(syn::Token![#]) {
                return Err(input.error("Cannot concatenate '#'"));
            }

            if fork.parse::<proc_macro2::Group>().is_ok() {
                return Err(input.error("Expected Non-Group Token"));
            }
            input.parse::<proc_macro2::TokenTree>()
        };

        let mut ret = ConcatSection {
            tokens: vec![],
        };

        eprintln!("parse_concat: pre-visit: {:?}", input);
        if !start_cond(input) {
            // try parsing another way if possible
            return ParseOutcome::RecoverableError;
        }

        match input.parse::<proc_macro2::TokenTree>() {
            Ok(tt) => ret.tokens.push(tt),
            Err(err) => return ParseOutcome::FatalError(err),
        }
        while !stop_cond(input) {
            match parse_concat_link(input) {
                Ok(tt) => ret.tokens.push(tt),
                Err(err) => return ParseOutcome::FatalError(err),
            }
        }
        eprintln!("parse_concat: post-visit: {:?}", input);
        ParseOutcome::Valid(ret)
    }

    fn parse_group(&self, input: syn::parse::ParseStream) -> ParseOutcome<Group> {

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
            return ParseOutcome::RecoverableError;
        }

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return ParseOutcome::RecoverableError;
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<Group> {

            let mut ret = Group {
                sections: vec![],
            };

            while !input.is_empty() {

                let fork = &input.fork();
                match self.parse_identity(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_identity(input) {
                            ret.sections.push(SectionTree::Identity(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }

                match try_extract::<ConcatSection>(self, input) {
                    Ok(ParseOutcome::Valid(sect)) => {
                        ret.sections.push(SectionTree::Concat(sect));
                        continue;
                    }
                    Ok(ParseOutcome::RecoverableError) => {},
                    Ok(ParseOutcome::FatalError(err)) => return Err(err),
                    Err(err) => return Err(err),
                }

                match try_extract::<ReplaceSection>(self, input) {
                    Ok(ParseOutcome::Valid(sect)) => {
                        ret.sections.push(SectionTree::Replace(sect));
                        continue;
                    }
                    Ok(ParseOutcome::RecoverableError) => {},
                    Ok(ParseOutcome::FatalError(err)) => return Err(err),
                    Err(err) => return Err(err),
                }

                // let fork = &input.fork();
                // match self.parse_concat(fork) {
                //     ParseOutcome::Valid(_) => {
                //         if let ParseOutcome::Valid(sect) = self.parse_concat(input) {
                //             ret.sections.push(SectionTree::Concat(sect));
                //         } else {
                //             panic!("Fork parsed but input not parsed. This should never happen");
                //         }
                //         continue;
                //     }
                //     // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Concat(sect)),
                //     ParseOutcome::FatalError(err) => return Err(err),
                //     ParseOutcome::RecoverableError => {},
                // }

                // let fork = &input.fork();
                // match self.parse_replace(fork) {
                //     ParseOutcome::Valid(_) => {
                //         if let ParseOutcome::Valid(sect) = self.parse_replace(input) {
                //             ret.sections.push(SectionTree::Replace(sect));
                //         } else {
                //             panic!("Fork parsed but input not parsed. This should never happen");
                //         }
                //         continue;
                //     }
                //     // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Concat(sect)),
                //     ParseOutcome::FatalError(err) => return Err(err),
                //     ParseOutcome::RecoverableError => {},
                // }

                let fork = &input.fork();
                match self.parse_group(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_group(input) {
                            ret.sections.push(SectionTree::Group(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Group(sect)),
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }
                
                // No branch matched, Unrecoverable
                return Err(input.error("Unable to Parse"));
            }

            Ok(ret)
        };

        let ret = parser.parse2(g.stream());
        eprintln!("parse_group: post-visit: {:?}", input);

        match ret {
            Ok(group) => ParseOutcome::Valid(group),
            Err(error) => ParseOutcome::FatalError(error),
        }
    }

    fn parse_identity_group(&self, input: syn::parse::ParseStream) -> ParseOutcome<IdentityGroup> {

        let start_cond = |input: syn::parse::ParseStream| -> bool {
            let fork = input.fork();
            if input.is_empty()
            || fork.parse::<proc_macro2::Group>().is_err() {
                return false;
            }
            true
        };

        eprintln!("identity_group: pre-visit: {:?}", input);
        if !start_cond(input) {
            // try parsing another way if possible
            return ParseOutcome::RecoverableError;
        }

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return ParseOutcome::RecoverableError;
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<IdentityGroup> {

            let mut ret = IdentityGroup {
                sections: vec![],
            };

            while !input.is_empty() {

                let fork = &input.fork();
                match self.parse_identity(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_identity(input) {
                            ret.sections.push(SectionTree::Identity(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }

                let fork = &input.fork();
                match self.parse_identity_group(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_identity_group(input) {
                            ret.sections.push(SectionTree::IdentityGroup(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Group(sect)),
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }

                // No branch matched, Unrecoverable
                return Err(input.error("Identity Group: Unable to Parse"));
            }

            Ok(ret)
        };

        let ret = parser.parse2(g.stream());
        eprintln!("parse_group: post-visit: {:?}", input);

        match ret {
            Ok(group) => ParseOutcome::Valid(group),
            Err(error) => ParseOutcome::FatalError(error),
        }
    }

    fn parse_no_nest_repeat_group(&self, input: syn::parse::ParseStream) -> ParseOutcome<NoNestRepeatGroup> {

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
            return ParseOutcome::RecoverableError;
        }

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return ParseOutcome::RecoverableError;
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<NoNestRepeatGroup> {

            let mut ret = NoNestRepeatGroup {
                sections: vec![],
            };

            while !input.is_empty() {

                let fork = &input.fork();
                match self.parse_identity(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_identity(input) {
                            ret.sections.push(SectionTree::Identity(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }

                match try_extract::<ConcatSection>(self, input) {
                    Ok(ParseOutcome::Valid(sect)) => {
                        ret.sections.push(SectionTree::Concat(sect));
                        continue;
                    }
                    Ok(ParseOutcome::RecoverableError) => {},
                    Ok(ParseOutcome::FatalError(err)) => return Err(err),
                    Err(err) => return Err(err),
                }

                match try_extract::<ReplaceSection>(self, input) {
                    Ok(ParseOutcome::Valid(sect)) => {
                        ret.sections.push(SectionTree::Replace(sect));
                        continue;
                    }
                    Ok(ParseOutcome::RecoverableError) => {},
                    Ok(ParseOutcome::FatalError(err)) => return Err(err),
                    Err(err) => return Err(err),
                }

                // let fork = &input.fork();
                // match self.parse_replace(fork) {
                //     ParseOutcome::Valid(_) => {
                //         if let ParseOutcome::Valid(sect) = self.parse_replace(input) {
                //             ret.sections.push(SectionTree::Replace(sect));
                //         } else {
                //             panic!("Fork parsed but input not parsed. This should never happen");
                //         }
                //         continue;
                //     }
                //     // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Concat(sect)),
                //     ParseOutcome::FatalError(err) => return Err(err),
                //     ParseOutcome::RecoverableError => {},
                // }

                let fork = &input.fork();
                match self.parse_no_nest_repeat_group(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_no_nest_repeat_group(input) {
                            ret.sections.push(SectionTree::NoNestRepeatGroup(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Group(sect)),
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }
                
                // No branch matched, Unrecoverable
                return Err(input.error("Unable to Parse"));
            }

            Ok(ret)
        };

        let ret = parser.parse2(g.stream());
        eprintln!("parse_group: post-visit: {:?}", input);

        match ret {
            Ok(no_nest_repeat_group) => ParseOutcome::Valid(no_nest_repeat_group),
            Err(error) => ParseOutcome::FatalError(error),
        }
    }

    fn parse_repeat_group(&self, input: syn::parse::ParseStream) -> ParseOutcome<RepeatGroup> {

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
            return ParseOutcome::RecoverableError;
        }
        let _ = input.parse::<syn::Token![#]>();

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return ParseOutcome::RecoverableError;
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<RepeatGroup> {

            let mut ret = RepeatGroup {
                sections: vec![],
            };

            while !input.is_empty() {

                let fork = &input.fork();
                match self.parse_identity(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_identity(input) {
                            ret.sections.push(SectionTree::Identity(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }

                match try_extract::<ConcatSection>(self, input) {
                    Ok(ParseOutcome::Valid(sect)) => {
                        ret.sections.push(SectionTree::Concat(sect));
                        continue;
                    }
                    Ok(ParseOutcome::RecoverableError) => {},
                    Ok(ParseOutcome::FatalError(err)) => return Err(err),
                    Err(err) => return Err(err),
                }

                match try_extract::<ReplaceSection>(self, input) {
                    Ok(ParseOutcome::Valid(sect)) => {
                        ret.sections.push(SectionTree::Replace(sect));
                        continue;
                    }
                    Ok(ParseOutcome::RecoverableError) => {},
                    Ok(ParseOutcome::FatalError(err)) => return Err(err),
                    Err(err) => return Err(err),
                }

                let fork = &input.fork();
                match self.parse_no_nest_repeat_group(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_no_nest_repeat_group(input) {
                            ret.sections.push(SectionTree::NoNestRepeatGroup(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Group(sect)),
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }
                
                // No branch matched, Unrecoverable
                return Err(input.error("RepeatGroup: Unable to Parse"));
            }

            Ok(ret)
        };

        let ret = parser.parse2(g.stream());
        eprintln!("parse_repeat_group: post-visit: {:?}", input);

        match ret {
            Ok(repeat_group) => ParseOutcome::Valid(repeat_group),
            Err(error) => ParseOutcome::FatalError(error),
        }
    }

    fn parse_base_nest_repeat_group(&self, input: syn::parse::ParseStream) -> ParseOutcome<BaseNestRepeatGroup> {

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
            return ParseOutcome::RecoverableError;
        }

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return ParseOutcome::RecoverableError;
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<BaseNestRepeatGroup> {

            let mut ret = BaseNestRepeatGroup {
                sections: vec![],
            };

            while !input.is_empty() {

                let fork = &input.fork();
                match self.parse_identity(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_identity(input) {
                            ret.sections.push(SectionTree::Identity(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }

                let fork = &input.fork();
                match self.parse_identity_group(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_identity_group(input) {
                            ret.sections.push(SectionTree::IdentityGroup(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Group(sect)),
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }

                let fork = &input.fork();
                match self.parse_repeat_group(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_repeat_group(input) {
                            ret.sections.push(SectionTree::RepeatGroup(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Group(sect)),
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }
                
                // No branch matched, Unrecoverable
                return Err(input.error("BaseNestRepeatGroup: Unable to Parse"));
            }

            Ok(ret)
        };

        let ret = parser.parse2(g.stream());
        eprintln!("parse_base_nest_repeat_group: post-visit: {:?}", input);

        match ret {
            Ok(repeat_group) => ParseOutcome::Valid(repeat_group),
            Err(_error) => ParseOutcome::RecoverableError,
        }
    }

    fn parse_base_no_nest_repeat_group(&self, input: syn::parse::ParseStream) -> ParseOutcome<BaseNoNestRepeatGroup> {

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
            return ParseOutcome::RecoverableError;
        }

        let Ok(g) = input.parse::<proc_macro2::Group>() else {
            return ParseOutcome::RecoverableError;
        };

        let parser = |input: syn::parse::ParseStream| -> syn::parse::Result<BaseNoNestRepeatGroup> {

            let mut ret = BaseNoNestRepeatGroup {
                sections: vec![],
            };

            while !input.is_empty() {

                let fork = &input.fork();
                match self.parse_identity(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_identity(input) {
                            ret.sections.push(SectionTree::Identity(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }

                match try_extract::<ConcatSection>(self, input) {
                    Ok(ParseOutcome::Valid(sect)) => {
                        ret.sections.push(SectionTree::Concat(sect));
                        continue;
                    }
                    Ok(ParseOutcome::RecoverableError) => {},
                    Ok(ParseOutcome::FatalError(err)) => return Err(err),
                    Err(err) => return Err(err),
                }

                match try_extract::<ReplaceSection>(self, input) {
                    Ok(ParseOutcome::Valid(sect)) => {
                        ret.sections.push(SectionTree::Replace(sect));
                        continue;
                    }
                    Ok(ParseOutcome::RecoverableError) => {},
                    Ok(ParseOutcome::FatalError(err)) => return Err(err),
                    Err(err) => return Err(err),
                }

                // let fork = &input.fork();
                // match self.parse_concat(fork) {
                //     ParseOutcome::Valid(_) => {
                //         if let ParseOutcome::Valid(sect) = self.parse_concat(input) {
                //             ret.sections.push(SectionTree::Concat(sect));
                //         } else {
                //             panic!("Fork parsed but input not parsed. This should never happen");
                //         }
                //         continue;
                //     }
                //     // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Concat(sect)),
                //     ParseOutcome::FatalError(err) => return Err(err),
                //     ParseOutcome::RecoverableError => {},
                // }

                // let fork = &input.fork();
                // match self.parse_replace(fork) {
                //     ParseOutcome::Valid(_) => {
                //         if let ParseOutcome::Valid(sect) = self.parse_replace(input) {
                //             ret.sections.push(SectionTree::Replace(sect));
                //         } else {
                //             panic!("Fork parsed but input not parsed. This should never happen");
                //         }
                //         continue;
                //     }
                //     // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Concat(sect)),
                //     ParseOutcome::FatalError(err) => return Err(err),
                //     ParseOutcome::RecoverableError => {},
                // }

                let fork = &input.fork();
                match self.parse_group(fork) {
                    ParseOutcome::Valid(_) => {
                        if let ParseOutcome::Valid(sect) = self.parse_group(input) {
                            ret.sections.push(SectionTree::Group(sect));
                        } else {
                            panic!("Fork parsed but input not parsed. This should never happen");
                        }
                        continue;
                    }
                    // ParseOutcome::Valid(sect) => ret.sections.push(SectionTree::Group(sect)),
                    ParseOutcome::FatalError(err) => return Err(err),
                    ParseOutcome::RecoverableError => {},
                }
                
                // No branch matched, Unrecoverable
                return Err(input.error("Unable to Parse"));
            }

            Ok(ret)
        };

        let ret = parser.parse2(g.stream());
        eprintln!("parse_repeat_group: post-visit: {:?}", input);

        match ret {
            Ok(repeat_group) => ParseOutcome::Valid(repeat_group),
            Err(_error) => ParseOutcome::RecoverableError,
        }
    }
}

#[derive(Debug)]
struct SeqTree {
    parse_ctx: ParseContext,
}

impl syn::parse::Parse for SeqTree {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let parse_ctx: ParseContext = input.parse()?;
        // eprintln!("parse_ctx is {:?}", parse_ctx);

        let fork = &input.fork();
        match parse_ctx.parse_base_nest_repeat_group(fork) {
            ParseOutcome::Valid(_) => {
                let ParseOutcome::Valid(group) = parse_ctx.parse_base_nest_repeat_group(input) else {
                    panic!("Fork parsed but input not parsed. This should never happen");
                };
                eprintln!("base_nest_repeat_group: {:?}\n", group);
                return Ok(SeqTree{
                    parse_ctx,
                })
            },
            ParseOutcome::RecoverableError => eprintln!("Cannot parse as BaseNestRepeatGroup"),
            ParseOutcome::FatalError(err) => return Err(err),
        }

        let fork = &input.fork();
        match parse_ctx.parse_base_no_nest_repeat_group(fork) {
            ParseOutcome::Valid(_) => {
                let ParseOutcome::Valid(group) = parse_ctx.parse_base_no_nest_repeat_group(input) else {
                    panic!("Fork parsed but input not parsed. This should never happen");
                };
                eprintln!("base_no_nest_repeat_group: {:?}\n", group);
                return Ok(SeqTree{
                    parse_ctx,
                })
            },
            ParseOutcome::RecoverableError => eprintln!("Cannot parse as BaseNoNestRepeatGroup"),
            ParseOutcome::FatalError(err) => return Err(err),
        }

        // let fork = &input.fork();
        // match parse_ctx.parse_group(fork) {
        //     ParseOutcome::Valid(_) => {
        //         let ParseOutcome::Valid(group) = parse_ctx.parse_group(input) else {
        //             panic!("Fork parsed but input not parsed. This should never happen");
        //         };
        //         eprintln!("group: {:?}\n", group);
        //     },
        //     ParseOutcome::RecoverableError => eprintln!("Cannot parse as Group"),
        //     ParseOutcome::FatalError(err) => return Err(err),
        // }

        // let group = parse_ctx.parse_group(input)?;
        // eprintln!("group is {:?}\n", group);

        Ok(SeqTree{
            parse_ctx,
        })
    }
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {

    // let input = input.parse::<syn::parse::ParseStream>()?;
    
    let seq_tree = syn::parse_macro_input!(input as SeqTree);
    
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
    
    TokenStream::new()
}
