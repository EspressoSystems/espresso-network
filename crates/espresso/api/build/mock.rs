//! A test stand-in for the node's state, producing `espresso.api.v2.mock.rs` in the build's
//! `OUT_DIR`: every v2 service trait implemented for `MockV2State`, each method answering
//! `INTERNAL`. Generated from the descriptor so that a new rpc needs no hand-written stub.

use std::fmt::Write as _;

use prost::Message as _;
use prost_types::FileDescriptorSet;

use crate::PACKAGE;

pub fn generate(descriptor_bytes: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let fdset = FileDescriptorSet::decode(descriptor_bytes)?;
    let mut out = String::from("#[derive(Clone)]\npub(crate) struct MockV2State;\n");
    let services = fdset
        .file
        .iter()
        .filter(|file| file.package.as_deref() == Some(PACKAGE))
        .flat_map(|file| &file.service);
    for service in services {
        writeln!(
            out,
            "\n#[tonic::async_trait]\nimpl crate::proto::{}_server::{} for MockV2State {{",
            snake_case(service.name()),
            service.name()
        )?;
        for method in &service.method {
            let input = proto_path(method.input_type());
            let output = proto_path(method.output_type());
            let response = if method.server_streaming() {
                let stream = format!("{}Stream", method.name());
                writeln!(
                    out,
                    "    type {stream} = futures::stream::BoxStream<'static, Result<{output}, \
                     tonic::Status>>;"
                )?;
                format!("Self::{stream}")
            } else {
                output
            };
            writeln!(
                out,
                "    async fn {}(&self, _request: tonic::Request<{input}>) -> \
                 Result<tonic::Response<{response}>, tonic::Status> {{\n        \
                 Err(tonic::Status::internal(\"mock\"))\n    }}",
                snake_case(method.name())
            )?;
        }
        out.push_str("}\n");
    }
    Ok(out)
}

/// `.espresso.api.v2.GetLeafRequest` as the generated type `crate::proto::GetLeafRequest`.
fn proto_path(type_name: &str) -> String {
    let name = type_name.rsplit('.').next().unwrap_or(type_name);
    format!("crate::proto::{name}")
}

/// tonic's method and module names: an underscore before each capital that follows a lowercase
/// letter or a digit, so `GetStateCertV2` becomes `get_state_cert_v2`.
fn snake_case(name: &str) -> String {
    let mut out = String::new();
    let mut prev: Option<char> = None;
    for c in name.chars() {
        if c.is_ascii_uppercase()
            && prev.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
        {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
        prev = Some(c);
    }
    out
}
