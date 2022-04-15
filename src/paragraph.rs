use nom::{IResult, bytes::complete::take_until, error::Error};

use crate::space::space;

pub fn paragraph(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, _) = space(input)?;
    match take_until::<&str, &[u8], Error<&[u8]>>("\\par")(input){
        Ok((input, par)) => Ok((&input[4..], par)),
        Err(_) => Ok((&[], input)),
    }
}

#[cfg(test)]
mod test {
    
    use super::*;

    #[test]
    fn simple_paragraph() {
        let input = b"25 \\par";

        let (input, par) = paragraph(input).unwrap();

        assert!(input.is_empty());
        assert_eq!(par, b"25 ");
    }

    #[test]
    fn no_par() {
        let input = &b"25 "[..];

        let (input, par) = paragraph(input).unwrap();

        assert_eq!(par, b"25 ");
        assert!(input.is_empty());
    }

    #[test]
    fn empty() {
        let input = b"    \\par";

        let (input, par) = paragraph(input).unwrap();

        assert!(input.is_empty());
        assert_eq!(par, b"");
    }

    #[test]
    fn eof() {
        let input = b"2022";

        let (input, year) = paragraph(input).unwrap();

        assert!(input.is_empty());
        assert_eq!(year, b"2022");
    }
}
