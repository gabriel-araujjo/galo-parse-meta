use nom::{bytes::complete::take_while, IResult};

pub fn space(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = take_while(|c| match c {
        b' ' | b'\t' | b'\r' | b'\n' => true,
        _ => false,
    })(input)?;

    Ok((input, ()))
}
