//! A deprecated proto field, message, rpc or enum value is flagged in the OpenAPI document, which is
//! how a REST client learns that a renamed field's old name, or a retired endpoint, is going away.
//!
//! No committed proto deprecates anything yet, so the fixture is the real descriptor set with an
//! item or two marked deprecated.

use prost::Message as _;
use prost_types::{
    DescriptorProto, EnumValueOptions, FieldOptions, FileDescriptorSet, MessageOptions,
    MethodOptions,
};
use serde_json::{Value, json};

/// `openapi` reads this for the package it generates from.
const PACKAGE: &str = "espresso.api.v2";

#[path = "../build/openapi.rs"]
#[expect(
    dead_code,
    reason = "the binding guards are covered by `openapi_guards`"
)]
mod openapi;

/// The OpenAPI document for the real descriptor after `edit`.
///
/// The edit is made on the decoded set: re-encoding a prost-types descriptor would drop the
/// google.api.http bindings, and with them every route.
fn spec(edit: impl FnOnce(&mut FileDescriptorSet)) -> Value {
    let rest_fdset =
        tonic_rest_build::descriptor::FileDescriptorSet::decode(espresso_api::FILE_DESCRIPTOR_SET)
            .unwrap();
    let mut fdset = FileDescriptorSet::decode(espresso_api::FILE_DESCRIPTOR_SET).unwrap();
    edit(&mut fdset);
    openapi::generate_from(&fdset, &rest_fdset).unwrap()
}

fn message_mut<'a>(fdset: &'a mut FileDescriptorSet, name: &str) -> &'a mut DescriptorProto {
    fdset
        .file
        .iter_mut()
        .flat_map(|file| file.message_type.iter_mut())
        .find(|message| message.name() == name)
        .unwrap_or_else(|| panic!("message {name} exists"))
}

fn deprecate_message(fdset: &mut FileDescriptorSet, name: &str) {
    message_mut(fdset, name).options = Some(MessageOptions {
        deprecated: Some(true),
        ..Default::default()
    });
}

fn deprecate_field(fdset: &mut FileDescriptorSet, message: &str, field: &str) {
    message_mut(fdset, message)
        .field
        .iter_mut()
        .find(|f| f.name() == field)
        .unwrap_or_else(|| panic!("field {message}.{field} exists"))
        .options = Some(FieldOptions {
        deprecated: Some(true),
        ..Default::default()
    });
}

fn deprecate_rpc(fdset: &mut FileDescriptorSet, service: &str, rpc: &str) {
    fdset
        .file
        .iter_mut()
        .flat_map(|file| file.service.iter_mut())
        .filter(|s| s.name() == service)
        .flat_map(|s| s.method.iter_mut())
        .find(|method| method.name() == rpc)
        .unwrap_or_else(|| panic!("rpc {service}.{rpc} exists"))
        .options = Some(MethodOptions {
        deprecated: Some(true),
        ..Default::default()
    });
}

fn deprecate_enum_value(fdset: &mut FileDescriptorSet, enum_name: &str, value: &str) {
    fdset
        .file
        .iter_mut()
        .flat_map(|file| file.enum_type.iter_mut())
        .filter(|e| e.name() == enum_name)
        .flat_map(|e| e.value.iter_mut())
        .find(|v| v.name() == value)
        .unwrap_or_else(|| panic!("enum value {enum_name}.{value} exists"))
        .options = Some(EnumValueOptions {
        deprecated: Some(true),
        ..Default::default()
    });
}

fn operation<'a>(spec: &'a Value, operation_id: &str) -> &'a Value {
    spec["paths"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|path| path.as_object().unwrap().values())
        .find(|op| op["operationId"] == operation_id)
        .unwrap_or_else(|| panic!("operation {operation_id} exists"))
}

fn schema<'a>(spec: &'a Value, name: &str) -> &'a Value {
    &spec["components"]["schemas"][name]
}

#[test]
fn nothing_is_flagged_while_no_proto_deprecates_anything() {
    let spec = spec(|_| {});
    assert!(!spec.to_string().contains("\"deprecated\""));
}

#[test]
fn a_deprecated_query_parameter_is_flagged() {
    let spec = spec(|fdset| deprecate_field(fdset, "GetHeaderRequest", "height"));
    let flagged = operation(&spec, "GetHeader")["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|param| param["deprecated"] == json!(true))
        .map(|param| param["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(flagged, ["height"]);
}

#[test]
fn a_deprecated_scalar_property_is_flagged() {
    let spec = spec(|fdset| deprecate_field(fdset, "TotalMintedSupplyResponse", "amount"));
    let amount = &schema(&spec, "TotalMintedSupplyResponse")["properties"]["amount"];
    assert_eq!(amount["deprecated"], json!(true));
    assert_eq!(amount["type"], json!("string"));
}

#[test]
fn a_deprecated_reference_is_wrapped_so_the_flag_is_not_ignored() {
    let spec = spec(|fdset| deprecate_field(fdset, "SyncStatusResponse", "blocks"));
    let properties = &schema(&spec, "SyncStatusResponse")["properties"];
    assert_eq!(
        properties["blocks"],
        json!({
            "allOf": [{"$ref": "#/components/schemas/ResourceSyncStatus"}],
            "deprecated": true,
        })
    );
    assert_eq!(
        properties["leaves"],
        json!({"$ref": "#/components/schemas/ResourceSyncStatus"})
    );
}

#[test]
fn a_deprecated_message_is_flagged() {
    let spec = spec(|fdset| deprecate_message(fdset, "SyncStatusResponse"));
    assert_eq!(
        schema(&spec, "SyncStatusResponse")["deprecated"],
        json!(true)
    );
    assert_eq!(
        schema(&spec, "ResourceSyncStatus")["deprecated"],
        Value::Null
    );
}

#[test]
fn a_deprecated_rpc_is_flagged() {
    let spec = spec(|fdset| deprecate_rpc(fdset, "AvailabilityService", "GetHeader"));
    assert_eq!(operation(&spec, "GetHeader")["deprecated"], json!(true));
    assert_eq!(operation(&spec, "GetLeaf")["deprecated"], Value::Null);
}

#[test]
fn a_deprecated_enum_value_is_noted_in_the_enum_description() {
    let spec = spec(|fdset| {
        deprecate_enum_value(fdset, "BuilderType", "BUILDER_TYPE_UNSPECIFIED");
        deprecate_enum_value(fdset, "BuilderType", "BUILDER_TYPE_RANDOM");
    });
    let description = schema(&spec, "BuilderType")["description"]
        .as_str()
        .unwrap();
    assert_eq!(
        description.lines().collect::<Vec<_>>(),
        [
            "- `BUILDER_TYPE_UNSPECIFIED`: Deprecated.",
            "- `BUILDER_TYPE_EXTERNAL`: Blocks come from the builders at `builder_urls`",
            "- `BUILDER_TYPE_SIMPLE`: Each node runs its own builder",
            "- `BUILDER_TYPE_RANDOM`: Deprecated. Each node runs a builder that produces random \
             transactions",
        ]
    );
}
