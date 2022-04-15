use nom::{
    branch::{alt},
    bytes::complete::{is_not, tag},
    character::complete::char,
    IResult, sequence::{separated_pair, tuple}, error::ErrorKind,
};

use crate::space::space;

#[derive(Debug, PartialEq)]
pub struct Author<'a> {
    pub given: &'a[u8],
    pub family: &'a[u8],
}

fn name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, name) = is_not(&b",.\\"[..])(input)?;

    // skip end delim if necessary
    let input = if input.is_empty() { 
        input 
    } else {
        match input[0] {
            b',' | b'.' => &input[1..],
            _ => input,
        }
    };
    Ok((input, name))
}

enum AuthorPart<'a> {
    Given(&'a[u8]),
    Family(&'a[u8]),
}

fn author_part(input: &[u8]) -> IResult<&[u8], AuthorPart> {
    let key = alt((tag("given"), tag("family")));
    fn separator(input: &[u8]) -> IResult<&[u8], ()> {
        let (input, _) = space(input)?;
        let (input, _) = char('>')(input)?;
        space(input)
    }
    
    let (input, _) = space(input)?;
    let (input, (key, value)) = separated_pair(key, separator, name)(input)?;

    let part = match key {
        b"given" => AuthorPart::Given(value),
        b"family" => AuthorPart::Family(value),
        _ => unreachable!(),
    };

    Ok((input, part))
}

pub fn author(input: &[u8]) -> IResult<&[u8], Author> {
    let original_input = input;
    let (input, parts) = tuple((author_part, author_part))(input)?;

    let (given, family) = match parts {
        (AuthorPart::Family(family), AuthorPart::Given(given)) => (given, family),
        (AuthorPart::Given(given), AuthorPart::Family(family)) => (given, family),
        _ => return Err(nom::Err::Error(nom::error::Error::new(original_input, ErrorKind::Satisfy))),
    };

    Ok((input, Author { given, family }))
}

#[cfg(test)]
mod test {

    use super::{author, Author};

    #[test]
    fn no_space() {
        let input = b"given>Fulano de,family>Tal";

        let (input, author) = author(input).unwrap();

        assert_eq!(
            author,
            Author {
                family: b"Tal",
                given: b"Fulano de",
            }
        );

        assert!(input.is_empty());
    }

    #[test]
    fn spaced() {
        let input = br#"
            given > Fulano de.
            family > Tal.
        "#;

        let (input, author) = author(input).unwrap();

        assert_eq!(
            author,
            Author {
                family: b"Tal",
                given: b"Fulano de",
            }
        );

        assert!(!input.is_empty());
    }

    #[test]
    fn end_par() {
        let input = br#"
            given > Fulano de.
            family > Tal\par"#;
            
        let (input, author) = author(input).unwrap();
        
        assert_eq!(
            author,
            Author {
                family: b"Tal",
                given: b"Fulano de",
            }
        );
        
        assert_eq!(input, b"\\par");
    }
}
