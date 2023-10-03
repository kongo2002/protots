use nom::branch::alt;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_while1;
use nom::character::complete::alphanumeric1;
use nom::character::complete::char;
use nom::character::complete::digit1;
use nom::character::complete::multispace0;
use nom::character::complete::multispace1;
use nom::character::complete::space0;
use nom::character::complete::space1;
use nom::combinator::map_res;
use nom::combinator::opt;
use nom::combinator::recognize;
use nom::multi::many0;
use nom::multi::separated_list0;
use nom::sequence::delimited;
use nom::IResult;

use crate::errors;
use crate::errors::PtError;

#[derive(Debug)]
pub struct Proto {
    pub syntax: String,
    pub elems: Vec<Elem>,
}

#[derive(Debug)]
pub enum Flag {
    None,
    Optional,
    Repeated,
}

#[derive(Debug)]
pub enum Field {
    Single {
        name: String,
        field_type: String,
        idx: u32,
        flag: Flag,
    },
    OneOf {
        name: String,
        fields: Vec<Field>,
    },
    Reserved {
        idx: u32,
    },
}

#[derive(Debug)]
pub struct Rpc {
    pub name: String,
    pub request: String,
    pub response: String,
    pub stream: bool,
}

#[derive(Debug)]
pub enum EnumValue {
    Single { name: String, idx: u32 },
    Reserved { idx: u32 },
}

#[derive(Debug)]
pub enum Elem {
    Message {
        name: String,
        fields: Vec<Field>,
    },
    Enum {
        name: String,
        values: Vec<EnumValue>,
    },
    Option {
        name: String,
        value: String,
    },
    Import {
        name: String,
    },
    Package {
        name: String,
    },
    SingleLineComment,
    Service {
        name: String,
        endpoints: Vec<Rpc>,
    },
}

fn import(input: &str) -> IResult<&str, Elem> {
    let (input, _) = tag("import")(input)?;
    let (input, _) = space1(input)?;
    let (input, import) = str(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((
        input,
        Elem::Import {
            name: import.to_string(),
        },
    ))
}

fn package(input: &str) -> IResult<&str, Elem> {
    let (input, _) = tag("package")(input)?;
    let (input, _) = space1(input)?;
    let (input, package) = is_not(";")(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((
        input,
        Elem::Package {
            name: package.to_string(),
        },
    ))
}

fn single_line_comment(input: &str) -> IResult<&str, Elem> {
    let (input, _) = tag("//")(input)?;
    let (input, _) = take_while1(|chr| chr != '\r' && chr != '\n')(input)?;

    Ok((input, Elem::SingleLineComment))
}

fn option(input: &str) -> IResult<&str, Elem> {
    let (input, _) = tag("option")(input)?;
    let (input, _) = space1(input)?;
    let (input, option_name) = not_space(input)?;
    let (input, _) = space1(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = space1(input)?;
    let (input, value) = is_not(";")(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((
        input,
        Elem::Option {
            name: option_name.to_string(),
            value: value.to_string(),
        },
    ))
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

fn field_flag(input: &str) -> IResult<&str, Flag> {
    let (input, flag0) = opt(alt((tag("optional"), tag("repeated"))))(input)?;
    let (input, _) = space0(input)?;
    let flag = match flag0 {
        Some("optional") => Flag::Optional,
        Some("repeated") => Flag::Repeated,
        _ => Flag::None,
    };

    Ok((input, flag))
}

fn enum_reserved_value(input: &str) -> IResult<&str, EnumValue> {
    let (input, _) = tag("reserved")(input)?;
    let (input, _) = space1(input)?;
    let (input, idx) = number(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((input, EnumValue::Reserved { idx }))
}

fn enum_value(input: &str) -> IResult<&str, EnumValue> {
    let (input, name) = not_space(input)?;
    let (input, _) = space1(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = space0(input)?;
    let (input, idx) = number(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((
        input,
        EnumValue::Single {
            name: name.to_string(),
            idx,
        },
    ))
}

fn enum0(input: &str) -> IResult<&str, Elem> {
    let (input, _) = tag("enum")(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = alphanumeric1(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("{")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, values) =
        separated_list0(multispace0, alt((enum_reserved_value, enum_value)))(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("}")(input)?;

    Ok((
        input,
        Elem::Enum {
            name: name.to_string(),
            values,
        },
    ))
}

fn oneof(input: &str) -> IResult<&str, Field> {
    let (input, _) = tag("oneof")(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = alphanumeric1(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("{")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, fields) = separated_list0(multispace1, field)(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("}")(input)?;

    Ok((
        input,
        Field::OneOf {
            name: name.to_string(),
            fields,
        },
    ))
}

fn message_field(input: &str) -> IResult<&str, Field> {
    let (input, flag) = field_flag(input)?;
    let (input, field_type) = not_space(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = not_space(input)?;
    let (input, _) = space1(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = space1(input)?;
    let (input, idx) = number(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((
        input,
        Field::Single {
            field_type: field_type.to_string(),
            name: name.to_string(),
            idx,
            flag,
        },
    ))
}

fn message_field_reserved(input: &str) -> IResult<&str, Field> {
    let (input, _) = tag("reserved")(input)?;
    let (input, _) = space1(input)?;
    let (input, idx) = number(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((input, Field::Reserved { idx }))
}

fn field(input: &str) -> IResult<&str, Field> {
    alt((oneof, message_field_reserved, message_field))(input)
}

fn rpc(input: &str) -> IResult<&str, Rpc> {
    let (input, _) = tag("rpc")(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = alphanumeric1(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("(")(input)?;
    let (input, _) = space0(input)?;
    let (input, request) = alphanumeric1(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag(")")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("returns")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("(")(input)?;
    let (input, _) = space0(input)?;
    let (input, stream) = opt(tag("stream"))(input)?;
    let (input, _) = space0(input)?;
    let (input, response) = alphanumeric1(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag(")")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("{")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("}")(input)?;

    Ok((
        input,
        Rpc {
            name: name.to_string(),
            request: request.to_string(),
            response: response.to_string(),
            stream: stream.is_some(),
        },
    ))
}

fn service(input: &str) -> IResult<&str, Elem> {
    let (input, _) = tag("service")(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = alphanumeric1(input)?;
    let (input, _) = space1(input)?;
    let (input, _) = tag("{")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, rpcs) = separated_list0(multispace1, rpc)(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("}")(input)?;

    Ok((
        input,
        Elem::Service {
            name: name.to_string(),
            endpoints: rpcs,
        },
    ))
}

fn message(input: &str) -> IResult<&str, Elem> {
    let (input, _) = tag("message")(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = alphanumeric1(input)?;
    let (input, _) = space1(input)?;
    let (input, _) = tag("{")(input)?;
    let (input, _) = ws(input)?;
    let (input, fields) = separated_list0(ws, field)(input)?;
    let (input, _) = ws(input)?;
    let (input, _) = tag("}")(input)?;

    Ok((
        input,
        Elem::Message {
            name: name.to_string(),
            fields,
        },
    ))
}

fn number(input: &str) -> IResult<&str, u32> {
    map_res(recognize(digit1), str::parse)(input)
}

fn not_space(input: &str) -> IResult<&str, &str> {
    is_not(" \r\n\t")(input)
}

fn ws(input: &str) -> IResult<&str, ()> {
    let comment = |i| {
        let (i, _) = tag("//")(i)?;
        take_while1(|chr| chr != '\r' && chr != '\n')(i)
    };
    let (input, _) = many0(alt((multispace1, comment)))(input)?;

    Ok((input, ()))
}

fn str(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), is_not("\""), char('"'))(input)
}

fn parse0(input: &str) -> IResult<&str, Proto> {
    let (input, _) = ws(input)?;
    let (input, syntax) = syntax(input)?;
    let (input, _) = ws(input)?;
    let (input, elems) = separated_list0(
        ws,
        alt((
            import,
            option,
            package,
            message,
            enum0,
            service,
            single_line_comment,
        )),
    )(input)?;
    let (input, _) = ws(input)?;

    Ok((
        input,
        Proto {
            syntax: syntax.to_string(),
            elems,
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
