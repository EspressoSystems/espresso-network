//! OpenAPI 3.0 generation from the compiled proto descriptor set, producing
//! `src/generated/espresso.api.v2.openapi.json`.
//!
//! The schemas must track what the pbjson impls emit, which is not a proto type's natural JSON:
//! a `uint64` is a decimal string in a body but plain digits in a query parameter.

use std::collections::{BTreeMap, BTreeSet};

use prost::Message as _;
use prost_types::{
    DescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileDescriptorSet,
    field_descriptor_proto::{Label, Type},
};
use serde_json::{Value, json};

use crate::PACKAGE;

/// Messages of this package by fully-qualified name, with their file's comments and their index in
/// that file (which is how source-code-info keys their field comments).
type Messages<'a> = BTreeMap<String, (&'a DescriptorProto, Comments, usize)>;

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
            if !message.nested_type.is_empty() {
                // A nested type is never registered as a schema, so a field referencing one would
                // emit a `$ref` to a schema that does not exist. Neither this generator nor the
                // REST transcoder handles them, so refuse rather than publish a broken document.
                return Err(format!(
                    "{}: nested messages are not supported in the v2 API",
                    message.name()
                )
                .into());
            }
            let name = message.name().to_string();
            // Schemas and the reachability walk both key on the short name, so a collision would
            // silently drop one message's schema and prune the other's.
            if schemas
                .insert(name.clone(), message_schema(message, &comments, i))
                .is_some()
            {
                return Err(format!("duplicate message name `{name}` in {PACKAGE}").into());
            }
            messages.insert(format!(".{PACKAGE}.{name}"), (message, comments.clone(), i));
        }
    }

    let mut paths: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
    let mut operation_ids = BTreeSet::new();
    let mut referenced = BTreeSet::new();
    for file in &package_files {
        let comments = Comments::new(file);
        for (si, service) in file.service.iter().enumerate() {
            for (mi, method) in service.method.iter().enumerate() {
                let key = (service.name().to_string(), method.name().to_string());
                // An rpc with no google.api.http annotation is gRPC-only and has no REST route
                // to document.
                let Some((verb, path)) = routes.get(&key) else {
                    continue;
                };
                if !operation_ids.insert(method.name().to_string()) {
                    return Err(format!(
                        "duplicate operationId `{}`: rpc names must be unique across services",
                        method.name()
                    )
                    .into());
                }
                let operation = operation(
                    service.name(),
                    method,
                    &comments.get(&[6, si as i32, 2, mi as i32]),
                    &messages,
                )?;
                if paths
                    .entry(path.clone())
                    .or_default()
                    .insert(verb.clone(), operation)
                    .is_some()
                {
                    return Err(format!("duplicate route: {verb} {path}").into());
                }
                reachable_schemas(method.output_type(), &messages, &mut referenced);
            }
        }
    }

    // Request messages are query parameters, not bodies, so nothing can `$ref` them. Publishing
    // them anyway leaves a client generator with a type per endpoint that it never uses.
    schemas.retain(|name, _| referenced.contains(name));
    schemas.insert("Error".to_string(), error_schema());

    Ok(json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Espresso Node API v2",
            "description": "Generated from the proto definitions in crates/espresso/api/proto/v2. \
                            JSON follows canonical protoJSON: camelCase field names, 64-bit \
                            integers as decimal strings, bytes as base64, oneofs flattened, \
                            defaults omitted. Query parameters accept both the proto field name \
                            and its camelCase form.",
            "version": "2",
        },
        "paths": paths,
        "components": { "schemas": schemas },
    }))
}

/// Records `type_name` and every message reachable from its fields, which is the set a client
/// needs to deserialize a response.
fn reachable_schemas(type_name: &str, messages: &Messages, out: &mut BTreeSet<String>) {
    let short = short_name(type_name);
    if !out.insert(short.to_string()) {
        return;
    }
    let Some((message, ..)) = messages.get(type_name) else {
        return;
    };
    for field in &message.field {
        if field.r#type() == Type::Message {
            reachable_schemas(field.type_name(), messages, out);
        }
    }
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
    messages: &Messages,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut op = json!({
        "tags": [service.strip_suffix("Service").unwrap_or(service)],
        "operationId": method.name(),
        "parameters": request_parameters(method.input_type(), messages)?,
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
        let summary = comment.lines().next().unwrap_or_default();
        op["summary"] = json!(summary);
        // Only when it says more than the summary, so UIs do not render the same line twice.
        if comment.trim() != summary {
            op["description"] = json!(comment);
        }
    }
    Ok(op)
}

/// Request message fields become query parameters, named after the proto field.
fn request_parameters(
    input_type: &str,
    messages: &Messages,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Only messages of this package are registered, so a miss means the rpc takes something this
    // generator cannot describe (an imported or well-known type). Emitting an empty parameter list
    // would silently drop every parameter from the docs.
    let Some((message, comments, index)) = messages.get(input_type) else {
        return Err(format!("request type {input_type} is not a message of {PACKAGE}").into());
    };
    let params: Vec<Value> = message
        .field
        .iter()
        .enumerate()
        .map(|(j, field)| {
            let mut param = json!({
                "name": field.name(),
                "in": "query",
                // Proto3 has no required fields: an absent parameter decodes to its default, so
                // the server accepts every subset. Whether a default is *meaningful* is the rpc's
                // business, not the schema's.
                "required": false,
                "schema": query_schema(field),
            });
            if let Some(comment) = comments.get(&[4, *index as i32, 2, j as i32]) {
                param["description"] = json!(comment);
            }
            param
        })
        .collect();
    Ok(json!(params))
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

/// The encoding pbjson emits for this field in a response body.
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
    json!({ "$ref": format!("#/components/schemas/{}", short_name(type_name)) })
}

fn short_name(type_name: &str) -> &str {
    type_name.rsplit('.').next().unwrap_or(type_name)
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
        Self { by_path }
    }

    fn get(&self, path: &[i32]) -> Option<String> {
        self.by_path.get(path).cloned()
    }
}
