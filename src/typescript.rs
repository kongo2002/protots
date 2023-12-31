use std::collections::HashMap;

use crate::errors::PtError;
use crate::parser::{Elem, Enum, EnumValue, Field, Flag, Msg, Proto};

const DEFAULT_CAPACITY: usize = 10 * 1024;

pub fn to_schema(proto: &Proto) -> Result<String, PtError> {
    let ctx = Context::new(proto);
    let mut str = String::with_capacity(DEFAULT_CAPACITY);

    str.push_str("//\n");
    str.push_str("// Code generated by protots - DO NOT EDIT\n");
    str.push_str(format!("// Source: {}\n", proto.file).as_str());
    str.push_str("//\n");
    str.push_str("\n");
    str.push_str("import { z } from \"zod\";");
    str.push_str("\n");
    str.push_str("\n");

    for elem in &proto.elems {
        match elem {
            Elem::Message(msg) => str.push_str(format_msg(&ctx, msg, None)?.as_str()),
            Elem::Enum(e) => str.push_str(format_enum(&ctx, e, None)?.as_str()),
            _ => (),
        }
    }

    Ok(str)
}

fn format_msg(ctx: &Context, msg: &Msg, parent: Option<&ProtoType>) -> Result<String, PtError> {
    let mut sub_messages = Vec::new();
    let mut fields = Vec::new();

    let ptype = ctx
        .get(&msg.name, parent)
        .ok_or(PtError::ProtobufTypeNotFound(msg.name.clone()))?;
    let message_name = &ptype.ts_name;

    for field in &msg.fields {
        if let Some(value) = format_field(ctx, field, Some(ptype), &mut sub_messages)? {
            fields.push(value);
        }
    }

    let mut str = String::with_capacity(512);

    for sub_msg in sub_messages {
        str.push_str(&sub_msg);
    }

    str.push_str(format!("export const {} = z.object({{\n", ptype.schema).as_str());
    for field in fields {
        str.push_str("  ");
        str.push_str(field.as_str());
        str.push_str(",\n");
    }
    str.push_str("});\n\n");

    str.push_str(
        format!(
            "export type {} = z.infer<typeof {}>;\n\n",
            message_name, ptype.schema
        )
        .as_str(),
    );

    Ok(str)
}

fn format_field(
    ctx: &Context,
    field: &Field,
    parent: Option<&ProtoType>,
    elements: &mut Vec<String>,
) -> Result<Option<String>, PtError> {
    match field {
        Field::Single {
            name,
            field_type,
            idx: _,
            flag,
        } => Ok(Some(format!(
            "{}: {}",
            snake_to_camel(name),
            flagged_field(type_name(ctx, &field_type, parent)?, flag)
        ))),
        Field::Map {
            name,
            key_type,
            value_type,
            idx: _,
        } => Ok(Some(format!(
            "{}: z.record({}, {})",
            snake_to_camel(name),
            type_name(ctx, key_type, parent)?,
            type_name(ctx, value_type, parent)?
        ))),
        Field::OneOf { name, fields } => Ok(Some(format!(
            "{}: {}",
            snake_to_camel(name),
            format_oneof(ctx, fields, parent, elements)?
        ))),
        Field::SubMessage(msg) => {
            elements.push(format_msg(ctx, msg, parent)?);
            Ok(None)
        }
        Field::SubEnum(e) => {
            elements.push(format_enum(ctx, e, parent)?);
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn format_oneof(
    ctx: &Context,
    oneof: &Vec<Field>,
    parent: Option<&ProtoType>,
    elements: &mut Vec<String>,
) -> Result<String, PtError> {
    let cases: Vec<_> = oneof
        .iter()
        .map(|case| format_field(ctx, case, parent, elements))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .map(|value| format!("z.object({{ {} }})", value))
        .collect();

    // z.union does not support single element lists
    if cases.len() == 1 {
        let single_field = &cases[0];
        return Ok(single_field.to_string());
    }

    Ok(format!("z.union([{}])", cases.join(", ")))
}

fn format_enum(ctx: &Context, value: &Enum, parent: Option<&ProtoType>) -> Result<String, PtError> {
    let mut str = String::with_capacity(512);
    let ptype = ctx
        .get(&value.name, parent)
        .ok_or(PtError::ProtobufTypeNotFound(value.name.clone()))?;
    let enum_name = &ptype.ts_name;

    str.push_str(format!("export enum {} {{\n", enum_name).as_str());

    for value in &value.values {
        match value {
            EnumValue::Single { name, idx: _ } => {
                str.push_str(format!("  {} = \"{}\",\n", name, name).as_str())
            }
            EnumValue::Reserved { idx: _ } => (),
        }
    }

    str.push_str("}\n\n");

    let default_case = value.values.iter().find_map(|value| match value {
        EnumValue::Single { name, idx } => {
            if *idx == 0 {
                Some(name)
            } else {
                None
            }
        }
        EnumValue::Reserved { idx: _ } => None,
    });

    let catch = default_case
        .map(|def_case| format!(".catch({}.{})", enum_name, def_case))
        .unwrap_or_else(|| String::new());

    str.push_str(
        format!(
            "export const {} = z.nativeEnum({}){};\n\n",
            ptype.schema, enum_name, catch
        )
        .as_str(),
    );

    Ok(str)
}

fn type_name<'a>(
    ctx: &'a Context,
    type_name: &'a str,
    parent: Option<&ProtoType>,
) -> Result<&'a str, PtError> {
    match type_name {
        // native types

        // strings
        "string" | "bytes" => Ok("z.string()"),
        // numbers
        "int32" | "double" | "float" | "uint32" | "sint32" | "fixed32" | "sfixed32" => {
            Ok("z.number()")
        }
        // bigint numbers
        "int64" | "uint64" | "fixed64" | "sfixed64" | "sint64" => Ok("z.coerce.bigint()"),

        // boolean
        "bool" => Ok("z.boolean()"),

        // external types
        "google.protobuf.Timestamp" => Ok("z.coerce.date()"),

        // try to lookup other types
        _ => ctx
            .get(type_name, parent)
            .map(|ptype| ptype.schema.as_str())
            .ok_or(PtError::ProtobufTypeNotFound(type_name.to_string())),
    }
}

fn flagged_field(field: &str, flag: &Flag) -> String {
    match flag {
        Flag::Optional => format!("z.optional({})", field),
        Flag::Repeated => format!("z.array({})", field),
        Flag::None => field.to_string(),
        Flag::Required => field.to_string(),
    }
}

fn to_camel(word: &str) -> String {
    let first_char = word.chars().nth(0);
    first_char
        .map(|first| {
            let mut new_word = Vec::with_capacity(word.len());
            new_word.push(first.to_ascii_uppercase());
            new_word.extend(word.chars().skip(1));
            new_word.into_iter().collect()
        })
        .unwrap_or_else(|| word.to_string())
}

fn snake_to_camel(input: &str) -> String {
    input
        .split('_')
        .filter(|part| !part.is_empty())
        .enumerate()
        .map(|(idx, part)| {
            if idx > 0 {
                to_camel(part)
            } else {
                part.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .concat()
}

struct ProtoType {
    full_name: String,
    ts_name: String,
    schema: String,
}

impl ProtoType {
    fn new(name: &str, parents: Vec<String>) -> ProtoType {
        let parts = parents
            .into_iter()
            .chain([name.to_string()])
            .collect::<Vec<_>>();
        let full_name = parts.join(".");
        let ts_name = parts.join("_");
        let schema = format!("{}Schema", ts_name);

        ProtoType {
            full_name,
            ts_name,
            schema,
        }
    }
}

struct Context {
    types: HashMap<String, ProtoType>,
}

impl Context {
    fn new(proto: &Proto) -> Context {
        let mut map = HashMap::new();

        for elem in &proto.elems {
            match elem {
                Elem::Message(msg) => {
                    map.insert(msg.name.clone(), ProtoType::new(&msg.name, Vec::new()));

                    for ptype in msg
                        .fields
                        .iter()
                        .flat_map(|fld| Self::collect(fld, vec![msg.name.clone()]))
                    {
                        map.insert(ptype.full_name.clone(), ptype);
                    }
                }
                Elem::Enum(e) => {
                    map.insert(e.name.clone(), ProtoType::new(&e.name, Vec::new()));
                }
                _ => (),
            }
        }

        Context { types: map }
    }

    fn get(&self, name: &str, parent: Option<&ProtoType>) -> Option<&ProtoType> {
        // first try the name as-is
        self.types
            .get(name)
            // then try with the parent's name prepended
            .or_else(|| {
                parent.and_then(|p| self.types.get(format!("{}.{}", p.full_name, name).as_str()))
            })
    }

    fn collect(field: &Field, mut parent: Vec<String>) -> Vec<ProtoType> {
        let mut types = Vec::new();
        match field {
            Field::SubMessage(msg) => {
                let ptype = ProtoType::new(&msg.name, parent.clone());
                types.push(ptype);

                parent.push(msg.name.clone());
                types.extend(
                    msg.fields
                        .iter()
                        .flat_map(|fld| Self::collect(fld, parent.clone())),
                );
            }
            Field::SubEnum(e) => types.push(ProtoType::new(&e.name, parent)),
            _ => (),
        }
        types
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::{Elem, Field, Msg, Proto};

    use super::to_schema;

    fn proto(elem: Elem) -> Proto {
        Proto {
            syntax: "proto3".to_string(),
            file: "file.proto".to_string(),
            elems: vec![elem],
        }
    }

    #[test]
    fn to_schema_single_oneof() {
        let p = proto(Elem::Message(Msg {
            name: "Test".to_string(),
            fields: vec![Field::OneOf {
                name: "test".to_string(),
                fields: vec![Field::Single {
                    name: "one".to_string(),
                    field_type: "string".to_string(),
                    idx: 1,
                    flag: crate::parser::Flag::None,
                }],
            }],
        }));

        let schema = to_schema(&p);
        assert_eq!(schema.is_ok(), true);
        assert_eq!(
            schema.unwrap(),
            r#"//
// Code generated by protots - DO NOT EDIT
// Source: file.proto
//

import { z } from "zod";

export const TestSchema = z.object({
  test: z.object({ one: z.string() }),
});

export type Test = z.infer<typeof TestSchema>;

"#
        );
    }

    #[test]
    fn to_schema_multiple_oneof() {
        let p = proto(Elem::Message(Msg {
            name: "Test".to_string(),
            fields: vec![Field::OneOf {
                name: "test".to_string(),
                fields: vec![
                    Field::Single {
                        name: "one".to_string(),
                        field_type: "string".to_string(),
                        idx: 1,
                        flag: crate::parser::Flag::None,
                    },
                    Field::Single {
                        name: "two".to_string(),
                        field_type: "int32".to_string(),
                        idx: 2,
                        flag: crate::parser::Flag::None,
                    },
                ],
            }],
        }));

        let schema = to_schema(&p);
        assert_eq!(schema.is_ok(), true);
        assert_eq!(
            schema.unwrap(),
            r#"//
// Code generated by protots - DO NOT EDIT
// Source: file.proto
//

import { z } from "zod";

export const TestSchema = z.object({
  test: z.union([z.object({ one: z.string() }), z.object({ two: z.number() })]),
});

export type Test = z.infer<typeof TestSchema>;

"#
        );
    }
}
