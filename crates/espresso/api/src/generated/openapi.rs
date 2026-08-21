//! OpenAPI 3.0 generation from the compiled proto descriptor set.
//!
//! Unlike its neighbors, this file is a hand-written `build.rs` module (the generator,
//! not an output); it produces `espresso.api.v2.openapi.json` alongside it.
//!
//! The document mirrors the protoJSON encoding produced by pbjson: lowerCamelCase
//! property names, 64-bit integers as decimal strings, bytes as base64, oneofs
//! flattened into the parent object, defaults omitted. Routes and HTTP methods come
//! from the `google.api.http` annotations; descriptions come from proto comments.

use std::collections::BTreeMap;

use prost::Message as _;
use prost_types::{
    DescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileDescriptorSet,
    field_descriptor_proto::{Label, Type},
};
use serde_json::{Value, json};

const PACKAGE: &str = "espresso.api.v2";

pub fn generate(descriptor_bytes: &[u8]) -> Result<Value, Box<dyn std::error::Error>> {
    let fdset = FileDescriptorSet::decode(descriptor_bytes)?;
    // The slim descriptor types from tonic-rest-core carry the google.api.http
    // extension that prost-types drops; decode the same bytes again for the routes.
    let rest_fdset = tonic_rest_build::descriptor::FileDescriptorSet::decode(descriptor_bytes)?;

    let routes = collect_routes(&rest_fdset);
    let package_files: Vec<&FileDescriptorProto> = fdset
        .file
        .iter()
        .filter(|f| f.package.as_deref() == Some(PACKAGE))
        .collect();

    let mut schemas = BTreeMap::new();
    let mut messages = BTreeMap::new();
    for file in &package_files {
        let comments = Comments::new(file);
        for (i, message) in file.message_type.iter().enumerate() {
            let name = message.name().to_string();
            schemas.insert(name.clone(), message_schema(message, &comments, i));
            messages.insert(format!(".{PACKAGE}.{name}"), (message, comments.clone()));
        }
    }
    schemas.insert("Error".to_string(), error_schema());

    let mut paths = BTreeMap::new();
    for file in &package_files {
        let comments = Comments::new(file);
        for (si, service) in file.service.iter().enumerate() {
            for (mi, method) in service.method.iter().enumerate() {
                let key = (service.name().to_string(), method.name().to_string());
                let Some((verb, path)) = routes.get(&key) else {
                    continue;
                };
                let operation = operation(
                    service.name(),
                    method,
                    &comments.get(&[6, si as i32, 2, mi as i32]),
                    &messages,
                );
                paths
                    .entry(path.clone())
                    .or_insert_with(BTreeMap::new)
                    .insert(verb.clone(), operation);
            }
        }
    }

    Ok(json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Espresso Node API v2",
            "description": "Generated from the proto definitions in crates/espresso/api/proto/v2. \
                            JSON follows canonical protoJSON: camelCase field names, 64-bit \
                            integers as decimal strings, bytes as base64, oneofs flattened, \
                            defaults omitted. Query parameters accept both the proto field name \
                            and its camelCase form.",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "paths": paths,
        "components": { "schemas": schemas },
    }))
}

/// `(service, method)` -> `(http verb, route)` from the `google.api.http` annotations.
fn collect_routes(
    fdset: &tonic_rest_build::descriptor::FileDescriptorSet,
) -> BTreeMap<(String, String), (String, String)> {
    let mut routes = BTreeMap::new();
    for file in &fdset.file {
        for service in &file.service {
            for method in &service.method {
                if let Some((verb, path)) =
                    tonic_rest_build::descriptor::extract_http_pattern(method)
                {
                    routes.insert(
                        (
                            service.name.clone().unwrap_or_default(),
                            method.name.clone().unwrap_or_default(),
                        ),
                        (verb.to_string(), path.to_string()),
                    );
                }
            }
        }
    }
    routes
}

fn operation(
    service: &str,
    method: &prost_types::MethodDescriptorProto,
    comment: &Option<String>,
    messages: &BTreeMap<String, (&DescriptorProto, Comments)>,
) -> Value {
    let mut op = json!({
        "tags": [service.trim_end_matches("Service")],
        "operationId": method.name(),
        "parameters": request_parameters(method.input_type(), messages),
        "responses": {
            "200": {
                "description": "OK",
                "content": { "application/json": { "schema": schema_ref(method.output_type()) } },
            },
            "default": {
                "description": "Error, following the Google API error model",
                "content": {
                    "application/json": { "schema": { "$ref": "#/components/schemas/Error" } },
                },
            },
        },
    });
    if let Some(comment) = comment {
        op["summary"] = json!(comment.lines().next().unwrap_or_default());
        op["description"] = json!(comment);
    }
    op
}

/// Request message fields become query parameters, named after the proto field
/// (the deserializer also accepts the camelCase form).
fn request_parameters(
    input_type: &str,
    messages: &BTreeMap<String, (&DescriptorProto, Comments)>,
) -> Value {
    let Some((message, comments)) = messages.get(input_type) else {
        return json!([]);
    };
    let message_index = message_index(messages, input_type);
    let params: Vec<Value> = message
        .field
        .iter()
        .enumerate()
        .map(|(j, field)| {
            let mut param = json!({
                "name": field.name(),
                "in": "query",
                "required": false,
                "schema": query_schema(field),
            });
            if let Some(comment) = comments.get(&[4, message_index, 2, j as i32]) {
                param["description"] = json!(comment);
            }
            param
        })
        .collect();
    json!(params)
}

fn message_index(messages: &BTreeMap<String, (&DescriptorProto, Comments)>, fqn: &str) -> i32 {
    messages
        .get(fqn)
        .map(|(m, c)| c.message_index(m.name()))
        .unwrap_or(0)
}

fn message_schema(message: &DescriptorProto, comments: &Comments, index: usize) -> Value {
    let mut properties = BTreeMap::new();
    for (j, field) in message.field.iter().enumerate() {
        let mut schema = field_schema(field);
        let mut notes = Vec::new();
        if let Some(comment) = comments.get(&[4, index as i32, 2, j as i32]) {
            notes.push(comment);
        }
        if let (Some(oneof), false) = (field.oneof_index, field.proto3_optional()) {
            let oneof_name = message
                .oneof_decl
                .get(oneof as usize)
                .map(|o| o.name().to_string())
                .unwrap_or_default();
            notes.push(format!("Member of oneof `{oneof_name}`."));
        }
        if !notes.is_empty() {
            let description = notes.join(" ");
            // OpenAPI 3.0 ignores siblings of $ref; wrap to keep the description.
            if schema.get("$ref").is_some() {
                schema = json!({ "allOf": [schema], "description": description });
            } else {
                schema["description"] = json!(description);
            }
        }
        properties.insert(field.json_name().to_string(), schema);
    }

    let mut schema = json!({ "type": "object", "properties": properties });
    if let Some(comment) = comments.get(&[4, index as i32]) {
        schema["description"] = json!(comment);
    }
    schema
}

/// protoJSON encoding of a message field, as produced by the pbjson impls.
fn field_schema(field: &FieldDescriptorProto) -> Value {
    let inner = match field.r#type() {
        Type::Message => schema_ref(field.type_name()),
        ty => scalar_schema(ty),
    };
    if field.label() == Label::Repeated {
        json!({ "type": "array", "items": inner })
    } else {
        inner
    }
}

/// Query parameters are not JSON: 64-bit integers arrive as plain digits, so
/// describe them as integers rather than protoJSON's string encoding.
fn query_schema(field: &FieldDescriptorProto) -> Value {
    match field.r#type() {
        Type::Int64 | Type::Sint64 | Type::Sfixed64 => {
            json!({ "type": "integer", "format": "int64" })
        },
        Type::Uint64 | Type::Fixed64 => json!({ "type": "integer", "format": "uint64" }),
        ty => scalar_schema(ty),
    }
}

fn scalar_schema(ty: Type) -> Value {
    match ty {
        Type::Double | Type::Float => json!({ "type": "number" }),
        Type::Int32 | Type::Sint32 | Type::Sfixed32 => {
            json!({ "type": "integer", "format": "int32" })
        },
        Type::Uint32 | Type::Fixed32 => json!({ "type": "integer", "format": "uint32" }),
        Type::Int64 | Type::Sint64 | Type::Sfixed64 => {
            json!({ "type": "string", "format": "int64" })
        },
        Type::Uint64 | Type::Fixed64 => json!({ "type": "string", "format": "uint64" }),
        Type::Bool => json!({ "type": "boolean" }),
        Type::Bytes => json!({ "type": "string", "format": "byte" }),
        _ => json!({ "type": "string" }),
    }
}

fn schema_ref(type_name: &str) -> Value {
    let short = type_name.rsplit('.').next().unwrap_or(type_name);
    json!({ "$ref": format!("#/components/schemas/{short}") })
}

/// The Google API error model emitted by `tonic_rest::RestError`.
fn error_schema() -> Value {
    json!({
        "type": "object",
        "description": "Error response following the Google API error model",
        "properties": {
            "error": {
                "type": "object",
                "properties": {
                    "code": { "type": "integer", "description": "HTTP status code" },
                    "message": { "type": "string" },
                    "status": { "type": "string", "description": "gRPC status name, e.g. NOT_FOUND" },
                },
            },
        },
    })
}

/// Leading proto comments, keyed by descriptor source-code-info path
/// (message i = [4, i], its field j = [4, i, 2, j]; service s = [6, s], its
/// method m = [6, s, 2, m]).
#[derive(Clone)]
struct Comments {
    by_path: BTreeMap<Vec<i32>, String>,
    message_names: Vec<String>,
}

impl Comments {
    fn new(file: &FileDescriptorProto) -> Self {
        let mut by_path = BTreeMap::new();
        if let Some(info) = &file.source_code_info {
            for location in &info.location {
                if let Some(comment) = &location.leading_comments {
                    let cleaned = comment
                        .lines()
                        .map(str::trim)
                        .collect::<Vec<_>>()
                        .join("\n")
                        .trim()
                        .to_string();
                    if !cleaned.is_empty() {
                        by_path.insert(location.path.clone(), cleaned);
                    }
                }
            }
        }
        let message_names = file
            .message_type
            .iter()
            .map(|m| m.name().to_string())
            .collect();
        Self {
            by_path,
            message_names,
        }
    }

    fn get(&self, path: &[i32]) -> Option<String> {
        self.by_path.get(path).cloned()
    }

    fn message_index(&self, name: &str) -> i32 {
        self.message_names
            .iter()
            .position(|n| n == name)
            .unwrap_or(0) as i32
    }
}
