use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::character::complete::alphanumeric1;
use nom::character::complete::char;
use nom::character::complete::multispace0;
use nom::character::complete::space0;
use nom::character::complete::space1;
use nom::multi::many0;
use nom::sequence::delimited;
use nom::IResult;

use crate::errors;
use crate::errors::PtError;

#[derive(Debug)]
pub struct Proto {
    pub syntax: String,
    pub messages: Vec<Message>,
}

#[derive(Debug)]
pub struct Message {
    pub name: String,
}

fn syntax(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("syntax")(input)?;
    let (input, _) = space1(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = space1(input)?;
    let (input, version) = str(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((input, version))
}

fn message(input: &str) -> IResult<&str, Message> {
    let (input, _) = tag("message")(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = alphanumeric1(input)?;
    let (input, _) = space1(input)?;
    let (input, _) = tag("{")(input)?;

    Ok((
        input,
        Message {
            name: name.to_string(),
        },
    ))
}

fn str(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), is_not("\""), char('"'))(input)
}

fn parse0(input: &str) -> IResult<&str, Proto> {
    let (input, syntax) = syntax(input)?;
    let (input, _) = multispace0(input)?;
    let (input, msgs) = many0(message)(input)?;

    Ok((
        input,
        Proto {
            syntax: syntax.to_string(),
            messages: msgs,
        },
    ))
}

pub fn parse(input: String) -> Result<(), PtError> {
    match parse0(&input) {
        Ok(("", proto)) => {
            println!("{:?}", proto);
            Ok(())
        }
        Ok((_, proto)) => {
            println!("{:?}", proto);
            Err(errors::PtError::IncompleteParsing)
        }
        Err(err) => {
            // TODO
            Err(errors::PtError::ParsingError(err.to_string()))
        }
    }
}
