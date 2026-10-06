//! The build refuses a proto it cannot describe. Every guard's passing direction runs on every
//! build, since the committed protos have to get through it; these cover the refusing direction,
//! which nothing else exercises.
//!
//! The fixture is the real descriptor set with one binding rewritten, so a guard is tested against
//! the shape it actually guards rather than a hand-built stub.

use prost::Message as _;
use tonic_rest_build::descriptor::{FileDescriptorSet, HttpPattern, HttpRule, MethodOptions};

/// `openapi` reads this for the package it generates from.
const PACKAGE: &str = "espresso.api.v2";

#[path = "../build/openapi.rs"]
mod openapi;

/// The real descriptor with the first rpc's binding replaced.
fn descriptor_with_binding(pattern: HttpPattern, body: &str) -> Vec<u8> {
    let mut fdset = FileDescriptorSet::decode(espresso_api::FILE_DESCRIPTOR_SET).unwrap();
    let method = fdset
        .file
        .iter_mut()
        .flat_map(|file| file.service.iter_mut())
        .flat_map(|service| service.method.iter_mut())
        .find(|method| method.options.is_some())
        .expect("some rpc carries an http binding");
    method.options = Some(MethodOptions {
        http: Some(HttpRule {
            pattern: Some(pattern),
            body: body.to_string(),
        }),
    });
    fdset.encode_to_vec()
}

#[test]
fn the_committed_protos_pass_their_own_guards() {
    openapi::check_bindings(espresso_api::FILE_DESCRIPTOR_SET).unwrap();
    openapi::generate(espresso_api::FILE_DESCRIPTOR_SET).unwrap();
}

#[test]
fn a_verb_other_than_get_or_post_is_refused() {
    let err = openapi::check_bindings(&descriptor_with_binding(
        HttpPattern::Put("/v2/node/anything".to_string()),
        "*",
    ))
    .expect_err("only GET and POST have a documented mapping");
    assert!(err.to_string().contains("only GET and POST"), "{err}");
}

#[test]
fn a_post_without_a_body_is_refused() {
    let err = openapi::check_bindings(&descriptor_with_binding(
        HttpPattern::Post("/v2/node/anything".to_string()),
        "",
    ))
    .expect_err("a POST's request is its body");
    assert!(err.to_string().contains("body: \"*\""), "{err}");
}

#[test]
fn a_get_with_a_body_is_refused() {
    let err = openapi::check_bindings(&descriptor_with_binding(
        HttpPattern::Get("/v2/node/anything".to_string()),
        "*",
    ))
    .expect_err("a GET's request is its query string");
    assert!(err.to_string().contains("cannot name a body"), "{err}");
}

#[test]
fn a_partial_body_is_refused() {
    let err = openapi::check_bindings(&descriptor_with_binding(
        HttpPattern::Post("/v2/node/anything".to_string()),
        "ranges",
    ))
    .expect_err("a body is the whole request message");
    assert!(err.to_string().contains("not the field `ranges`"), "{err}");
}

#[test]
fn a_path_template_is_refused() {
    let err = openapi::check_bindings(&descriptor_with_binding(
        HttpPattern::Get("/v2/node/anything/{height}".to_string()),
        "",
    ))
    .expect_err("a path template is documented as a query parameter");
    assert!(err.to_string().contains("path template"), "{err}");
}
