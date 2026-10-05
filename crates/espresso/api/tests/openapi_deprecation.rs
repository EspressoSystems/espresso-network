//! A deprecated proto field, message or rpc is flagged in the OpenAPI document, which is how a REST
//! client learns that a renamed field's old name, or a retired endpoint, is going away.
//!
//! No committed proto deprecates anything yet, so the fixture is the real descriptor set with a few
//! items marked deprecated.

use prost::Message as _;
use prost_types::{FieldOptions, FileDescriptorSet, MessageOptions, MethodOptions};
use serde_json::{Value, json};

/// `openapi` reads this for the package it generates from.
const PACKAGE: &str = "espresso.api.v2";

#[path = "../build/openapi.rs"]
#[expect(
    dead_code,
    reason = "the binding guards are covered by `openapi_guards`"
)]
mod openapi;

/// Which items of the real descriptor to mark deprecated.
#[derive(Default)]
struct Deprecate<'a> {
    /// As (message, field).
    fields: &'a [(&'a str, &'a str)],
    messages: &'a [&'a str],
    /// As (service, rpc).
    rpcs: &'a [(&'a str, &'a str)],
}

/// The OpenAPI document for the real descriptor with `deprecate` applied.
///
/// The edit is made on the decoded set: re-encoding a prost-types descriptor would drop the
/// google.api.http bindings, and with them every route.
fn spec(deprecate: Deprecate) -> Value {
    let Deprecate {
        fields,
        messages,
        rpcs,
    } = deprecate;
    let rest_fdset =
        tonic_rest_build::descriptor::FileDescriptorSet::decode(espresso_api::FILE_DESCRIPTOR_SET)
            .unwrap();
    let mut fdset = FileDescriptorSet::decode(espresso_api::FILE_DESCRIPTOR_SET).unwrap();
    for message in fdset
        .file
        .iter_mut()
        .flat_map(|file| file.message_type.iter_mut())
    {
        if messages.contains(&message.name()) {
            message.options = Some(MessageOptions {
                deprecated: Some(true),
                ..Default::default()
            });
        }
        let message_name = message.name().to_string();
        for field in &mut message.field {
            if fields.contains(&(message_name.as_str(), field.name())) {
                field.options = Some(FieldOptions {
                    deprecated: Some(true),
                    ..Default::default()
                });
            }
        }
    }
    for service in fdset
        .file
        .iter_mut()
        .flat_map(|file| file.service.iter_mut())
    {
        let service_name = service.name().to_string();
        for method in &mut service.method {
            if rpcs.contains(&(service_name.as_str(), method.name())) {
                method.options = Some(MethodOptions {
                    deprecated: Some(true),
                    ..Default::default()
                });
            }
        }
    }
    openapi::generate_from(&fdset, &rest_fdset).unwrap()
}

/// Every operation in `spec`, whatever its path and verb.
fn operations(spec: &Value) -> impl Iterator<Item = &Value> {
    spec["paths"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|path| path.as_object().unwrap().values())
}

fn operation<'a>(spec: &'a Value, operation_id: &str) -> &'a Value {
    operations(spec)
        .find(|op| op["operationId"] == operation_id)
        .unwrap_or_else(|| panic!("operation {operation_id} exists"))
}

fn schema<'a>(spec: &'a Value, name: &str) -> &'a Value {
    &spec["components"]["schemas"][name]
}

#[test]
fn nothing_is_flagged_while_no_proto_deprecates_anything() {
    let spec = spec(Deprecate::default());
    assert!(!spec.to_string().contains("\"deprecated\""));
}

#[test]
fn a_deprecated_query_parameter_is_flagged() {
    let spec = spec(Deprecate {
        fields: &[("GetHeaderRequest", "height")],
        ..Default::default()
    });
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
    let spec = spec(Deprecate {
        fields: &[("TotalMintedSupplyResponse", "amount")],
        ..Default::default()
    });
    let amount = &schema(&spec, "TotalMintedSupplyResponse")["properties"]["amount"];
    assert_eq!(amount["deprecated"], json!(true));
    assert_eq!(amount["type"], json!("string"));
}

#[test]
fn a_deprecated_reference_is_wrapped_so_the_flag_is_not_ignored() {
    let spec = spec(Deprecate {
        fields: &[("SyncStatusResponse", "blocks")],
        ..Default::default()
    });
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
    let spec = spec(Deprecate {
        messages: &["SyncStatusResponse"],
        ..Default::default()
    });
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
    let spec = spec(Deprecate {
        rpcs: &[("AvailabilityService", "GetHeader")],
        ..Default::default()
    });
    assert_eq!(operation(&spec, "GetHeader")["deprecated"], json!(true));
    assert_eq!(operation(&spec, "GetLeaf")["deprecated"], Value::Null);
}
