use nom::branch::alt;
use nom::bytes::complete::escaped;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_until;
use nom::bytes::complete::take_while;
use nom::character::complete::alpha1;
use nom::character::complete::alphanumeric1;
use nom::character::complete::char;
use nom::character::complete::multispace1;
use nom::character::complete::one_of;
use nom::character::complete::space0;
use nom::combinator::map_res;
use nom::combinator::opt;
use nom::combinator::recognize;
use nom::error::VerboseError;
use nom::multi::many0;
use nom::multi::many1;
use nom::multi::separated_list1;
use nom::sequence::delimited;
use nom::sequence::pair;
use nom::sequence::preceded;
use nom::IResult;

use crate::errors;
use crate::errors::PtError;

type ParserResult<'a, O> = IResult<&'a str, O, VerboseError<&'a str>>;

#[derive(Debug)]
pub struct Proto {
    pub file: String,
    pub syntax: String,
    pub elems: Vec<Elem>,
}

#[derive(Debug)]
pub enum Flag {
    None,
    Optional,
    Repeated,
    // proto2 only
    Required,
}

#[derive(Debug)]
pub enum ReservedField {
    Idx { idx: Vec<i32> },
    Name { name: Vec<String> },
}

#[derive(Debug)]
pub enum Field {
    Single {
        name: String,
        field_type: String,
        idx: i32,
        flag: Flag,
    },
    Map {
        name: String,
        key_type: String,
        value_type: String,
        idx: i32,
    },
    OneOf {
        name: String,
        fields: Vec<Field>,
    },
    SubMessage(Msg),
    SubEnum(Enum),
    Reserved(ReservedField),
    Extensions(String, String),
}

#[derive(Debug)]
pub struct Rpc {
    pub name: String,
    pub request: String,
    pub stream_request: bool,
    pub response: String,
    pub stream_response: bool,
}

#[derive(Debug)]
pub enum EnumValue {
    Single { name: String, idx: i32 },
    Reserved { idx: i32 },
}

#[derive(Debug)]
pub enum OptionValue {
    Str { value: String },
    Constant { value: String },
    Num { value: i32 },
    Bool { value: bool },
    Msg { value: String },
}

#[derive(Debug)]
pub struct Msg {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug)]
pub struct Enum {
    pub name: String,
    pub values: Vec<EnumValue>,
}

#[derive(Debug)]
pub struct Option {
    pub name: String,
    pub value: OptionValue,
}

#[derive(Debug)]
pub enum ServiceNode {
    Rpc(Rpc),
    Option(Option),
}

#[derive(Debug)]
pub enum Elem {
    Message(Msg),
    Enum(Enum),
    Option(Option),
    Import {
        name: String,
    },
    Package {
        name: String,
    },
    Extend {
        name: String,
        fields: Vec<Field>,
    },
    Service {
        name: String,
        nodes: Vec<ServiceNode>,
    },
}

fn import(input: &str) -> ParserResult<Elem> {
    let (input, _) = tag("import")(input)?;
    let (input, import) = ws(str)(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((
        input,
        Elem::Import {
            name: import.to_string(),
        },
    ))
}

fn package(input: &str) -> ParserResult<Elem> {
    let (input, _) = tag("package")(input)?;
    let (input, package) = ws(is_not(";"))(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((
        input,
        Elem::Package {
            name: package.to_string(),
        },
    ))
}

fn option_map_value(input: &str) -> ParserResult<&str> {
    let (input, _name) = identifier(input)?;
    let (input, _) = ws(tag(":"))(input)?;
    let (input, _value) = option_value(input)?;
    let (input, _) = opt(one_of(",;"))(input)?;

    Ok((input, ""))
}

fn option_value<'a>(input: &'a str) -> ParserResult<OptionValue> {
    let str = |i| {
        let (i, value) = str(i)?;
        Ok((
            i,
            OptionValue::Str {
                value: value.to_string(),
            },
        ))
    };
    let num = |i| {
        let (i, value) = number(i)?;
        Ok((i, OptionValue::Num { value }))
    };
    let bool = |i| {
        let (i, value) = boolean(i)?;
        Ok((i, OptionValue::Bool { value }))
    };
    let constant = |i: &'a str| {
        let (i, value) = alphanumeric1(i)?;
        Ok((
            i,
            OptionValue::Constant {
                value: value.to_string(),
            },
        ))
    };
    let msg = |i| {
        let (i, _) = tag("{")(i)?;
        let (i, _values) = many0(ws(option_map_value))(i)?;
        let (i, _) = ws(tag("}"))(i)?;
        Ok((
            i,
            OptionValue::Msg {
                // TODO
                value: "".to_string(),
            },
        ))
    };

    alt((str, num, bool, msg, constant))(input)
}

fn option_name(input: &str) -> ParserResult<&str> {
    let (input, _) = opt(tag("("))(input)?;
    let (input, val) = ws(identifier)(input)?;
    let (input, _) = opt(tag(")"))(input)?;

    Ok((input, val))
}

fn option(input: &str) -> ParserResult<Option> {
    let (input, _) = tag("option")(input)?;
    let (input, option_name) = ws(option_name)(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, value) = ws(option_value)(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((
        input,
        Option {
            name: option_name.to_string(),
            value,
        },
    ))
}

fn syntax(input: &str) -> ParserResult<&str> {
    let (input, _) = tag("syntax")(input)?;
    let (input, _) = ws(tag("="))(input)?;
    let (input, version) = ws(str)(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((input, version))
}

fn field_flag(input: &str) -> ParserResult<Flag> {
    let (input, flag0) = opt(alt((tag("optional"), tag("repeated"), tag("required"))))(input)?;
    let flag = match flag0 {
        Some("optional") => Flag::Optional,
        Some("repeated") => Flag::Repeated,
        Some("required") => Flag::Required,
        _ => Flag::None,
    };

    Ok((input, flag))
}

fn enum_reserved_value(input: &str) -> ParserResult<EnumValue> {
    let (input, _) = tag("reserved")(input)?;
    let (input, idx) = ws(number)(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((input, EnumValue::Reserved { idx }))
}

fn enum_value(input: &str) -> ParserResult<EnumValue> {
    let (input, name) = ws(identifier)(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, idx) = ws(number)(input)?;
    let (input, _) = opt(field_options)(input)?;
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

fn enum0(input: &str) -> ParserResult<Enum> {
    let (input, _) = tag("enum")(input)?;
    let (input, name) = ws(alphanumeric1)(input)?;
    let (input, _) = tag("{")(input)?;
    let (input, values) = many0(ws(alt((enum_reserved_value, enum_value))))(input)?;
    let (input, _) = ws(tag("}"))(input)?;
    let (input, _) = opt(tag(";"))(input)?;

    Ok((
        input,
        Enum {
            name: name.to_string(),
            values,
        },
    ))
}

fn proto_map(input: &str) -> ParserResult<Field> {
    let (input, _) = tag("map")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("<")(input)?;
    let (input, key_type) = ws(identifier)(input)?;
    let (input, _) = tag(",")(input)?;
    let (input, value_type) = ws(identifier)(input)?;
    let (input, _) = tag(">")(input)?;
    let (input, name) = ws(identifier)(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, idx) = ws(number)(input)?;
    let (input, _) = opt(field_options)(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((
        input,
        Field::Map {
            name: name.to_string(),
            key_type: key_type.to_string(),
            value_type: value_type.to_string(),
            idx,
        },
    ))
}

fn oneof(input: &str) -> ParserResult<Field> {
    let (input, _) = tag("oneof")(input)?;
    let (input, name) = ws(identifier)(input)?;
    let (input, _) = tag("{")(input)?;
    let (input, fields) = many0(ws(field))(input)?;
    let (input, _) = ws(tag("}"))(input)?;
    let (input, _) = opt(tag(";"))(input)?;

    Ok((
        input,
        Field::OneOf {
            name: name.to_string(),
            fields,
        },
    ))
}

fn extend(input: &str) -> ParserResult<Elem> {
    let (input, _) = tag("extend")(input)?;
    let (input, name) = ws(identifier)(input)?;
    let (input, _) = tag("{")(input)?;
    let (input, fields) = many0(ws(message_field))(input)?;
    let (input, _) = tag("}")(input)?;

    Ok((
        input,
        Elem::Extend {
            name: name.to_string(),
            fields,
        },
    ))
}

fn field_options(input: &str) -> ParserResult<()> {
    let (input, _) = tag("[")(input)?;
    let (input, _) = ws(option_name)(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = ws(option_value)(input)?;
    let (input, _) = tag("]")(input)?;

    Ok((input, ()))
}

fn message_field(input: &str) -> ParserResult<Field> {
    let (input, flag) = field_flag(input)?;
    let (input, field_type) = ws(identifier)(input)?;
    let (input, name) = ws(identifier)(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, idx) = ws(number)(input)?;
    let (input, _) = opt(field_options)(input)?;
    let (input, _) = space0(input)?;
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

fn extensions_field(input: &str) -> ParserResult<Field> {
    let (input, _) = tag("extensions")(input)?;
    let (input, from) = ws(alphanumeric1)(input)?;
    let (input, _) = tag("to")(input)?;
    let (input, to) = ws(alphanumeric1)(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((input, Field::Extensions(from.to_string(), to.to_string())))
}

fn reserved_field(input: &str) -> ParserResult<ReservedField> {
    let by_idx = map_res(separated_list1(ws(char(',')), number), |v| {
        Ok::<ReservedField, &str>(ReservedField::Idx { idx: v })
    });
    let by_name = map_res(separated_list1(ws(char(',')), str), |v| {
        Ok::<ReservedField, &str>(ReservedField::Name {
            name: v.into_iter().map(|v| v.to_string()).collect(),
        })
    });

    alt((by_idx, by_name))(input)
}

fn message_field_reserved(input: &str) -> ParserResult<Field> {
    let (input, _) = tag("reserved")(input)?;
    let (input, reserved) = ws(reserved_field)(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((input, Field::Reserved(reserved)))
}

fn field(input: &str) -> ParserResult<Field> {
    alt((
        oneof,
        message_field_reserved,
        message_field,
        proto_map,
        extensions_field,
        map_res(message, |v| Ok::<Field, &str>(Field::SubMessage(v))),
        map_res(enum0, |v| Ok::<Field, &str>(Field::SubEnum(v))),
    ))(input)
}

fn rpc_opts(input: &str) -> ParserResult<&str> {
    let (input, _) = tag("{")(input)?;
    let (input, _) = whitespace(input)?;
    let (input, _options) = many0(ws(option))(input)?;
    let (input, _) = ws(tag("}"))(input)?;

    Ok((input, ""))
}

fn rpc(input: &str) -> ParserResult<ServiceNode> {
    let (input, _) = tag("rpc")(input)?;
    let (input, name) = ws(alphanumeric1)(input)?;
    let (input, _) = ws(tag("("))(input)?;
    let (input, stream_request) = opt(tag("stream"))(input)?;
    let (input, request) = ws(identifier)(input)?;
    let (input, _) = ws(tag(")"))(input)?;
    let (input, _) = tag("returns")(input)?;
    let (input, _) = ws(tag("("))(input)?;
    let (input, stream_response) = opt(tag("stream"))(input)?;
    let (input, response) = ws(identifier)(input)?;
    let (input, _) = ws(tag(")"))(input)?;
    let (input, _) = opt(rpc_opts)(input)?;
    let (input, _) = opt(tag(";"))(input)?;

    Ok((
        input,
        ServiceNode::Rpc(Rpc {
            name: name.to_string(),
            request: request.to_string(),
            stream_request: stream_request.is_some(),
            response: response.to_string(),
            stream_response: stream_response.is_some(),
        }),
    ))
}

fn service_option(input: &str) -> ParserResult<ServiceNode> {
    let (input, opt) = option(input)?;
    Ok((input, ServiceNode::Option(opt)))
}

fn service(input: &str) -> ParserResult<Elem> {
    let (input, _) = tag("service")(input)?;
    let (input, name) = ws(alphanumeric1)(input)?;
    let (input, _) = ws(tag("{"))(input)?;
    let (input, _) = whitespace(input)?;
    let (input, nodes) = many0(ws(alt((rpc, service_option))))(input)?;
    let (input, _) = ws(tag("}"))(input)?;

    Ok((
        input,
        Elem::Service {
            name: name.to_string(),
            nodes,
        },
    ))
}

fn message(input: &str) -> ParserResult<Msg> {
    let (input, _) = tag("message")(input)?;
    let (input, name) = ws(alphanumeric1)(input)?;
    let (input, _) = ws(tag("{"))(input)?;
    let (input, fields) = many0(ws(field))(input)?;
    let (input, _) = ws(tag("}"))(input)?;
    let (input, _) = opt(tag(";"))(input)?;

    Ok((
        input,
        Msg {
            name: name.to_string(),
            fields,
        },
    ))
}

fn number(input: &str) -> ParserResult<i32> {
    map_res(recognize(many1(one_of("01234567890-"))), str::parse)(input)
}

fn boolean(input: &str) -> ParserResult<bool> {
    let (input, value) = alt((tag("true"), tag("false")))(input)?;
    let val = match value {
        "true" => true,
        "false" => false,
        _ => unreachable!(),
    };
    Ok((input, val))
}

// I don't know if this "has" to be that complicated...
fn ws<'a, T, F>(mut inner: F) -> impl FnMut(&'a str) -> ParserResult<T>
where
    F: FnMut(&'a str) -> ParserResult<T>,
{
    move |i| delimited(whitespace, &mut inner, whitespace)(i)
}

fn whitespace(input: &str) -> ParserResult<&str> {
    let single_line_comment = preceded(tag("//"), take_while(|chr| chr != '\r' && chr != '\n'));
    let multiline_comment = delimited(tag("/*"), take_until("*/"), tag("*/"));
    recognize(many0(alt((
        single_line_comment,
        multiline_comment,
        multispace1,
    ))))(input)
}

fn identifier(input: &str) -> ParserResult<&str> {
    recognize(pair(
        alpha1,
        many0(alt((alphanumeric1, tag("."), tag("_")))),
    ))(input)
}

fn str(input: &str) -> ParserResult<&str> {
    delimited(
        char('"'),
        escaped(is_not("\\\""), '\\', one_of("\"\n\r")),
        char('"'),
    )(input)
}

fn parse0<'a>(file_name: &'a str, input: &'a str) -> ParserResult<'a, Proto> {
    let (input, syntax) = ws(syntax)(input)?;
    let (input, elems) = many0(ws(alt((
        import,
        package,
        extend,
        map_res(option, |v| Ok::<Elem, &str>(Elem::Option(v))),
        map_res(message, |v| Ok::<Elem, &str>(Elem::Message(v))),
        map_res(enum0, |v| Ok::<Elem, &str>(Elem::Enum(v))),
        service,
    ))))(input)?;

    let fname = file_name.to_string();

    Ok((
        input,
        Proto {
            file: fname,
            syntax: syntax.to_string(),
            elems,
        },
    ))
}

pub fn parse(file_name: &str, input: &str) -> Result<Proto, PtError> {
    match parse0(file_name, input) {
        Ok(("", proto)) => Ok(proto),
        Ok((_, proto)) => {
            eprintln!("{:?}", proto);
            Err(errors::PtError::IncompleteParsing)
        }
        Err(err) => {
            // TODO
            Err(errors::PtError::ParsingError(err.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    const TEST_INPUT: &str = std::include_str!("../assets/example.proto");

    #[test]
    fn parse_example_file_is_ok() {
        let parsed = super::parse("", TEST_INPUT);
        assert_eq!(parsed.is_ok(), true);
    }
}
