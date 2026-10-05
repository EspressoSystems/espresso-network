//! Refuses a proto the v2 API cannot serve or document, before any code is generated from it.
//!
//! The generated REST handlers and the OpenAPI generator both assume the shapes checked here.

use std::collections::{BTreeMap, BTreeSet};

use prost_types::{
    DescriptorProto, FileDescriptorProto, FileDescriptorSet,
    field_descriptor_proto::{Label, Type},
};

use crate::PACKAGE;

/// `(service, method)` -> its route.
pub type Routes = BTreeMap<(String, String), Route>;

/// One `google.api.http` annotation.
pub struct Route {
    pub verb: String,
    pub path: String,
    /// The body selector, empty when the request is read from the query string.
    pub body: String,
}

/// A descriptor set that passed [`check`], with its routes. Only [`check`] constructs one, so
/// holding it is the proof the OpenAPI generator relies on.
pub struct Checked<'a> {
    fdset: &'a FileDescriptorSet,
    routes: Routes,
}

impl<'a> Checked<'a> {
    pub fn fdset(&self) -> &'a FileDescriptorSet {
        self.fdset
    }

    pub fn routes(&self) -> &Routes {
        &self.routes
    }
}

/// `fdset` and `rest_fdset` are the same descriptor set decoded twice: prost-types drops the
/// google.api.http extension that the slim tonic-rest types carry, and the slim types drop
/// everything else.
pub fn check<'a>(
    fdset: &'a FileDescriptorSet,
    rest_fdset: &tonic_rest_build::descriptor::FileDescriptorSet,
) -> Result<Checked<'a>, Box<dyn std::error::Error>> {
    let routes = collect_routes(rest_fdset);
    let package_files: Vec<&FileDescriptorProto> = fdset
        .file
        .iter()
        .filter(|f| f.package.as_deref() == Some(PACKAGE))
        .collect();

    check_bindings(&routes)?;
    check_types(&package_files)?;
    check_operations(&package_files, &routes)?;
    Ok(Checked { fdset, routes })
}

fn collect_routes(fdset: &tonic_rest_build::descriptor::FileDescriptorSet) -> Routes {
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

/// Refuse any binding the generator cannot describe.
///
/// A GET takes its request message as query parameters and a POST takes all of it as a JSON body
/// (`body: "*"`). Any other verb or body selector would be generated and documented as one of those
/// two, wrongly, so it fails the build. A path template fails for the same reason: every parameter
/// is documented `in: query`, so the document would never declare the template variable.
///
/// An `additional_bindings` block passes unnoticed, since the descriptor types do not decode it.
fn check_bindings(routes: &Routes) -> Result<(), Box<dyn std::error::Error>> {
    for ((service, method), Route { verb, path, body }) in routes {
        match (verb.as_str(), body.as_str()) {
            ("get", "") | ("post", "*") => {},
            ("get", _) => {
                return Err(format!(
                    "{service}.{method}: a GET takes its request as query parameters, so it \
                     cannot name a body"
                )
                .into());
            },
            ("post", "") => {
                return Err(format!(
                    "{service}.{method}: a POST takes the whole request message as its body, so \
                     bind it with `body: \"*\"`"
                )
                .into());
            },
            ("post", field) => {
                return Err(format!(
                    "{service}.{method}: a body selects the whole request message (`body: \
                     \"*\"`), not the field `{field}`"
                )
                .into());
            },
            _ => {
                return Err(format!(
                    "{service}.{method}: only GET and POST bindings are supported, not {verb}"
                )
                .into());
            },
        }
        if path.contains('{') {
            return Err(format!(
                "{service}.{method}: `{path}` has a path template; v2 addresses resources with \
                 query parameters, so give the route a constant path"
            )
            .into());
        }
    }
    Ok(())
}

/// Refuse a message or enum the document cannot name.
fn check_types(files: &[&FileDescriptorProto]) -> Result<(), Box<dyn std::error::Error>> {
    // Schemas and the reachability walk both key on the short name, so a collision would silently
    // drop one type's schema and prune the other's.
    let mut names = BTreeSet::new();
    for file in files {
        for message in &file.message_type {
            // A nested type is never registered as a schema, so a field referencing one would emit
            // a `$ref` to a schema that does not exist. Neither the generator nor the REST
            // transcoder handles them, so refuse rather than publish a broken document. A map entry
            // is the exception: it is synthesized by protoc and described inline as
            // `additionalProperties`.
            if message
                .nested_type
                .iter()
                .any(|nested| !nested.options.as_ref().is_some_and(|o| o.map_entry()))
            {
                return Err(format!(
                    "{}: nested messages are not supported in the v2 API",
                    message.name()
                )
                .into());
            }
            if !message.enum_type.is_empty() {
                return Err(format!(
                    "{}: nested enums are not supported in the v2 API",
                    message.name()
                )
                .into());
            }
            if !names.insert(message.name()) {
                return Err(
                    format!("duplicate message name `{}` in {PACKAGE}", message.name()).into(),
                );
            }
        }
        for enum_type in &file.enum_type {
            if !names.insert(enum_type.name()) {
                return Err(
                    format!("duplicate type name `{}` in {PACKAGE}", enum_type.name()).into(),
                );
            }
        }
    }
    Ok(())
}

/// Refuse an rpc whose REST route collides with another or whose query string cannot be decoded.
fn check_operations(
    files: &[&FileDescriptorProto],
    routes: &Routes,
) -> Result<(), Box<dyn std::error::Error>> {
    let messages: BTreeMap<String, &DescriptorProto> = files
        .iter()
        .flat_map(|file| &file.message_type)
        .map(|message| (format!(".{PACKAGE}.{}", message.name()), message))
        .collect();
    let mut operation_ids = BTreeSet::new();
    let mut verbs_and_paths = BTreeSet::new();
    for service in files.iter().flat_map(|file| &file.service) {
        for method in &service.method {
            // An rpc with no google.api.http annotation is gRPC-only and has no REST route.
            let Some(route) = routes.get(&(service.name().to_string(), method.name().to_string()))
            else {
                continue;
            };
            if !operation_ids.insert(method.name()) {
                return Err(format!(
                    "duplicate operationId `{}`: rpc names must be unique across services",
                    method.name()
                )
                .into());
            }
            if !verbs_and_paths.insert((&route.verb, &route.path)) {
                return Err(format!("duplicate route: {} {}", route.verb, route.path).into());
            }
            if route.body.is_empty() {
                check_query_message(method.input_type(), &messages)?;
            }
        }
    }
    Ok(())
}

/// A GET's request message is decoded from the query string.
fn check_query_message(
    input_type: &str,
    messages: &BTreeMap<String, &DescriptorProto>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Only messages of this package are documented, so a miss means the rpc takes something the
    // generator cannot describe (an imported or well-known type), and its parameters would
    // silently vanish from the docs.
    let Some(message) = messages.get(input_type) else {
        return Err(format!("request type {input_type} is not a message of {PACKAGE}").into());
    };
    for field in &message.field {
        // The generated handlers extract requests with `axum::extract::Query`, and
        // `serde_urlencoded` cannot decode a repeated or message-typed field, so such an rpc would
        // fail every request. Enum fields are refused by choice: one would decode by value name but
        // not by the number protoJSON also allows, and no endpoint wants one yet, so the
        // generator's `query_schema` has no inline enum branch. Add both together when one does.
        if field.label() == Label::Repeated || matches!(field.r#type(), Type::Message | Type::Enum)
        {
            return Err(format!(
                "{}.{}: request message fields must be scalars, since they are query parameters",
                message.name(),
                field.name()
            )
            .into());
        }
        // Without `optional` a scalar has implicit presence, so an omitted parameter arrives as
        // zero and the handler cannot tell it apart from a caller asking for zero. Marking every
        // one `optional` keeps that choice with the handler, which can then refuse the absence.
        if !field.proto3_optional() {
            return Err(format!(
                "{}.{}: request message fields must be `optional`, so an omitted parameter is \
                 distinguishable from a zero one",
                message.name(),
                field.name()
            )
            .into());
        }
    }
    Ok(())
}
