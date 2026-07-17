use std::{collections::HashMap, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("proto");
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

    let examples_path = proto_root.join("v2/examples.toml");
    let examples_content = std::fs::read_to_string(&examples_path)?;
    let examples: HashMap<String, HashMap<String, toml::Value>> =
        toml::from_str(&examples_content)?;

    let mut config = prost_build::Config::new();
    config.type_attribute(
        ".",
        "#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]",
    );

    config.field_attribute("Empty.dummy", "#[serde(skip)]");

    for (message_name, fields) in examples {
        for (field_name, value) in fields {
            let field_path = format!("{}.{}", message_name, field_name);
            let example_value = match value {
                toml::Value::String(s) => s,
                toml::Value::Integer(i) => i.to_string(),
                toml::Value::Float(f) => f.to_string(),
                toml::Value::Boolean(b) => b.to_string(),
                _ => continue,
            };
            config.field_attribute(
                &field_path,
                format!(r#"#[schemars(example = "{}")]"#, example_value),
            );
        }
    }

    config.out_dir(&out_dir).compile_protos(
        &[
            "v2/common.proto",
            "v2/rewards.proto",
            "v2/data.proto",
            "v2/consensus.proto",
        ],
        &[proto_root],
    )?;

    println!("cargo:rerun-if-changed=proto/v2/common.proto");
    println!("cargo:rerun-if-changed=proto/v2/rewards.proto");
    println!("cargo:rerun-if-changed=proto/v2/data.proto");
    println!("cargo:rerun-if-changed=proto/v2/consensus.proto");
    println!("cargo:rerun-if-changed=proto/v2/examples.toml");

    Ok(())
}
