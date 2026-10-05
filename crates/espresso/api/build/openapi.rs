//! OpenAPI 3.0 generation from the compiled proto descriptor set, producing
//! `espresso.api.v2.openapi.json` in the build's `OUT_DIR`.
//!
//! The schemas must track what the pbjson impls emit, which is not a proto type's natural JSON:
//! a `uint64` is a decimal string in a body but plain digits in a query parameter.
//!
//! The descriptor must have passed `guards::check`, which refuses every shape this module cannot
//! describe, so nothing here re-checks one.

use std::collections::{BTreeMap, BTreeSet};

use prost::Message as _;
use prost_types::{
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto,
    FileDescriptorSet,
    field_descriptor_proto::{Label, Type},
};
use serde_json::{Value, json};

use crate::PACKAGE;

/// Messages of this package by fully-qualified name, with their file's comments and their index in
/// that file (which is how source-code-info keys their field comments).
type Messages<'a> = BTreeMap<String, (&'a DescriptorProto, Comments, usize)>;

/// The synthetic entry message protoc generates for a `map` field, by fully-qualified name. Its
/// second field is the map's value type, which is what the field's schema describes.
type MapEntries<'a> = BTreeMap<String, &'a DescriptorProto>;

pub fn generate(descriptor_bytes: &[u8]) -> Result<Value, Box<dyn std::error::Error>> {
    let fdset = FileDescriptorSet::decode(descriptor_bytes)?;
    // The slim descriptor types from tonic-rest-core carry the google.api.http
    // extension that prost-types drops; decode the same bytes again for the routes.
    let rest_fdset = tonic_rest_build::descriptor::FileDescriptorSet::decode(descriptor_bytes)?;
    Ok(generate_from(&fdset, &rest_fdset))
}

/// [`generate`] over an already-decoded descriptor, read twice as there: `fdset` for messages and
/// comments, `rest_fdset` for the routes.
pub fn generate_from(
    fdset: &FileDescriptorSet,
    rest_fdset: &tonic_rest_build::descriptor::FileDescriptorSet,
) -> Value {
    let routes = collect_routes(rest_fdset);
    let package_files: Vec<&FileDescriptorProto> = fdset
        .file
        .iter()
        .filter(|f| f.package.as_deref() == Some(PACKAGE))
        .collect();

    let mut schemas = BTreeMap::new();
    let mut messages = BTreeMap::new();
    let mut map_entries = BTreeMap::new();
    // The only nested messages the guards let through are the map entries protoc synthesizes.
    for file in &package_files {
        for message in &file.message_type {
            for nested in &message.nested_type {
                map_entries.insert(
                    format!(".{PACKAGE}.{}.{}", message.name(), nested.name()),
                    nested,
                );
            }
        }
    }
    for file in &package_files {
        let comments = Comments::new(file);
        for (i, message) in file.message_type.iter().enumerate() {
            let name = message.name().to_string();
            schemas.insert(
                name.clone(),
                message_schema(message, &comments, i, &map_entries),
            );
            messages.insert(format!(".{PACKAGE}.{name}"), (message, comments.clone(), i));
        }
        for (i, enum_type) in file.enum_type.iter().enumerate() {
            schemas.insert(
                enum_type.name().to_string(),
                enum_schema(enum_type, &comments, i),
            );
        }
    }

    let mut paths: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
    let mut referenced = BTreeSet::new();
    for file in &package_files {
        let comments = Comments::new(file);
        for (si, service) in file.service.iter().enumerate() {
            for (mi, method) in service.method.iter().enumerate() {
                let key = (service.name().to_string(), method.name().to_string());
                // An rpc with no google.api.http annotation is gRPC-only and has no REST route
                // to document.
                let Some(route) = routes.get(&key) else {
                    continue;
                };
                let operation = operation(
                    service,
                    method,
                    route,
                    comments.get(&[6, si as i32, 2, mi as i32]),
                    &messages,
                );
                paths
                    .entry(route.path.clone())
                    .or_default()
                    .insert(route.verb.clone(), operation);
                reachable_schemas(
                    method.output_type(),
                    &messages,
                    &map_entries,
                    &mut referenced,
                );
                if !route.body.is_empty() {
                    reachable_schemas(
                        method.input_type(),
                        &messages,
                        &map_entries,
                        &mut referenced,
                    );
                }
            }
        }
    }

    // A GET's request message is inlined as query parameters, so nothing can `$ref` it. Publishing
    // it anyway leaves a client generator with a type per endpoint that it never uses. Its own
    // `deprecated` goes with it, so deprecate its fields, or the rpc, to reach a REST client.
    schemas.retain(|name, _| referenced.contains(name));
    schemas.insert("Error".to_string(), error_schema());

    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Espresso Node API v2",
            "description": "Generated from the proto definitions in crates/espresso/api/proto/v2. \
                            JSON follows canonical protoJSON: camelCase field names, 64-bit \
                            integers as decimal strings, bytes as base64, enums as their value \
                            names, oneofs flattened, defaults omitted. Query parameters accept \
                            both the proto field name \
                            and its camelCase form.",
            "version": "2",
        },
        "paths": paths,
        "components": { "schemas": schemas },
    })
}

/// Records `type_name` and every message and enum reachable from its fields, which is the set a
/// client needs to deserialize a response.
fn reachable_schemas(
    type_name: &str,
    messages: &Messages,
    map_entries: &MapEntries,
    out: &mut BTreeSet<String>,
) {
    // A map entry has no schema of its own, so it must not be recorded. Only its value type is
    // reachable from the document.
    if let Some(entry) = map_entries.get(type_name) {
        let value = &entry.field[1];
        if matches!(value.r#type(), Type::Message | Type::Enum) {
            reachable_schemas(value.type_name(), messages, map_entries, out);
        }
        return;
    }
    let short = short_name(type_name);
    if !out.insert(short.to_string()) {
        return;
    }
    let Some((message, ..)) = messages.get(type_name) else {
        return;
    };
    for field in &message.field {
        if matches!(field.r#type(), Type::Message | Type::Enum) {
            reachable_schemas(field.type_name(), messages, map_entries, out);
        }
    }
}

/// One `google.api.http` annotation.
pub struct Route {
    pub verb: String,
    pub path: String,
    /// The body selector, empty when the request is read from the query string.
    pub body: String,
}

/// `(service, method)` -> its route, from the `google.api.http` annotations.
pub fn collect_routes(
    fdset: &tonic_rest_build::descriptor::FileDescriptorSet,
) -> BTreeMap<(String, String), Route> {
    let mut routes = BTreeMap::new();
    for file in &fdset.file {
        for service in &file.service {
            for method in &service.method {
                let Some((verb, path)) = tonic_rest_build::descriptor::extract_http_pattern(method)
                else {
                    continue;
                };
                let body = method
                    .options
                    .as_ref()
                    .and_then(|options| options.http.as_ref())
                    .map_or(String::new(), |http| http.body.clone());
                routes.insert(
                    (
                        service.name.clone().unwrap_or_default(),
                        method.name.clone().unwrap_or_default(),
                    ),
                    Route {
                        verb: verb.to_string(),
                        path: path.to_string(),
                        body,
                    },
                );
            }
        }
    }
    routes
}

fn operation(
    service: &prost_types::ServiceDescriptorProto,
    method: &prost_types::MethodDescriptorProto,
    route: &Route,
    comment: Option<&str>,
    messages: &Messages,
) -> Value {
    // A server-streaming rpc is served as server-sent events, whose frames the schema describes.
    // Documented as `application/json`, a generated client would parse the stream as one object.
    let output = schema_ref(method.output_type());
    let ok = if method.server_streaming() {
        json!({
            "description": "Server-sent events: one `data:` frame per item holding the JSON of the \
                            response message, with keep-alive comments between items. An error \
                            is an `event: error` frame holding the error envelope, and ends the \
                            stream",
            "content": { "text/event-stream": { "schema": output } },
        })
    } else {
        json!({
            "description": "OK",
            "content": { "application/json": { "schema": output } },
        })
    };
    let has_body = !route.body.is_empty();
    let parameters = if has_body {
        json!([])
    } else {
        request_parameters(method.input_type(), messages)
    };
    let mut op = json!({
        "tags": [service.name().strip_suffix("Service").unwrap_or(service.name())],
        "operationId": method.name(),
        "parameters": parameters,
        "responses": {
            "200": ok,
            "default": {
                "description": "Error, following the Google API error model",
                "content": {
                    "application/json": { "schema": { "$ref": "#/components/schemas/Error" } },
                },
            },
        },
    });
    if has_body {
        op["requestBody"] = json!({
            "required": true,
            "content": { "application/json": { "schema": schema_ref(method.input_type()) } },
        });
    }
    if let Some(comment) = comment {
        // Unwrapped first: a proto comment is hard-wrapped, and a summary cut at the first line
        // break ends mid-sentence in the operation list every docs UI renders.
        let text = comment.split('\n').collect::<Vec<_>>().join(" ");
        let summary = match text.split_once(". ") {
            Some((first, _)) => format!("{first}."),
            None => text.clone(),
        };
        op["summary"] = json!(summary);
        // Only when it says more than the summary, so UIs do not render the same line twice.
        if text != summary {
            op["description"] = json!(text);
        }
    }
    // OpenAPI has no service object to flag, so a deprecated service marks each of its operations.
    if method.options.as_ref().is_some_and(|o| o.deprecated())
        || service.options.as_ref().is_some_and(|o| o.deprecated())
    {
        op["deprecated"] = json!(true);
    }
    op
}

/// Request message fields become query parameters, named after the proto field.
fn request_parameters(input_type: &str, messages: &Messages) -> Value {
    let (message, comments, index) = &messages[input_type];
    let mut params = Vec::new();
    for (j, field) in message.field.iter().enumerate() {
        let mut param = json!({
            "name": field.name(),
            "in": "query",
            // Every field is `optional`, so the schema cannot tell a parameter the handler
            // refuses to go without from one that means something when absent. The field's
            // description says which, and the handler answers 400 for the first kind.
            "required": false,
            "schema": query_schema(field),
        });
        if let Some(comment) = comments.get(&[4, *index as i32, 2, j as i32]) {
            param["description"] = json!(comment);
        }
        if field.options.as_ref().is_some_and(|o| o.deprecated()) {
            param["deprecated"] = json!(true);
        }
        params.push(param);
    }
    json!(params)
}

fn message_schema(
    message: &DescriptorProto,
    comments: &Comments,
    index: usize,
    map_entries: &MapEntries,
) -> Value {
    let mut properties = BTreeMap::new();
    for (j, field) in message.field.iter().enumerate() {
        let mut schema = field_schema(field, map_entries);
        let mut notes = Vec::new();
        if let Some(comment) = comments.get(&[4, index as i32, 2, j as i32]) {
            notes.push(comment.to_string());
        }
        if let (Some(oneof), false) = (field.oneof_index, field.proto3_optional()) {
            let oneof_name = message
                .oneof_decl
                .get(oneof as usize)
                .map(|o| o.name().to_string())
                .unwrap_or_default();
            notes.push(format!("Member of oneof `{oneof_name}`."));
        }
        let deprecated = field.options.as_ref().is_some_and(|o| o.deprecated());
        // OpenAPI 3.0 ignores siblings of $ref; wrap to keep the description and the flag.
        if (!notes.is_empty() || deprecated) && schema.get("$ref").is_some() {
            schema = json!({ "allOf": [schema] });
        }
        if !notes.is_empty() {
            schema["description"] = json!(notes.join(" "));
        }
        if deprecated {
            schema["deprecated"] = json!(true);
        }
        properties.insert(field.json_name().to_string(), schema);
    }

    let mut schema = json!({ "type": "object", "properties": properties });
    if let Some(comment) = comments.get(&[4, index as i32]) {
        schema["description"] = json!(comment);
    }
    if message.options.as_ref().is_some_and(|o| o.deprecated()) {
        schema["deprecated"] = json!(true);
    }
    schema
}

/// Enums reach the document only through responses, since the guards refuse enum request
/// fields. pbjson writes a value as its name, so publishing the names is all a client
/// needs to decode one.
fn enum_schema(enum_type: &EnumDescriptorProto, comments: &Comments, index: usize) -> Value {
    let values: Vec<&str> = enum_type.value.iter().map(|value| value.name()).collect();
    let mut schema = json!({ "type": "string", "enum": values });
    let mut sections = Vec::new();
    if let Some(comment) = comments.get(&[5, index as i32]) {
        sections.push(comment.to_string());
    }
    let value_notes: Vec<String> = enum_type
        .value
        .iter()
        .enumerate()
        .filter_map(|(j, value)| {
            let comment = comments.get(&[5, index as i32, 2, j as i32]);
            // OpenAPI 3.0 cannot flag a single enum value, so its note is the only place to say so.
            let note = match (
                value.options.as_ref().is_some_and(|o| o.deprecated()),
                comment,
            ) {
                (true, Some(comment)) => format!("Deprecated. {comment}"),
                (true, None) => "Deprecated.".to_string(),
                (false, Some(comment)) => comment.to_string(),
                (false, None) => return None,
            };
            Some(format!("- `{}`: {note}", value.name()))
        })
        .collect();
    if !value_notes.is_empty() {
        sections.push(value_notes.join("\n"));
    }
    if !sections.is_empty() {
        // Rendered as markdown by the docs UIs, where a list needs a blank line ahead of it and
        // single newlines collapse, running every value into one paragraph.
        schema["description"] = json!(sections.join("\n\n"));
    }
    if enum_type.options.as_ref().is_some_and(|o| o.deprecated()) {
        schema["deprecated"] = json!(true);
    }
    schema
}

/// The encoding pbjson emits for this field in a response body.
fn field_schema(field: &FieldDescriptorProto, map_entries: &MapEntries) -> Value {
    // A map is a repeated entry message on the wire but a JSON object, always keyed by a string
    // whatever the key's proto type.
    if let Some(entry) = map_entries.get(field.type_name()) {
        return json!({
            "type": "object",
            "additionalProperties": field_schema(&entry.field[1], map_entries),
        });
    }
    let inner = match field.r#type() {
        Type::Message | Type::Enum => schema_ref(field.type_name()),
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
/// (message i = [4, i], its field j = [4, i, 2, j]; enum e = [5, e], its value v = [5, e, 2, v];
/// service s = [6, s], its method m = [6, s, 2, m]).
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

    fn get(&self, path: &[i32]) -> Option<&str> {
        self.by_path.get(path).map(String::as_str)
    }
}
