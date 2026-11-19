fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure().compile_protos(
        &[
            "proto/espresso/consensus/v1/ingest.proto",
            "proto/espresso/consensus/v1/node.proto",
        ],
        &["proto"],
    )?;
    Ok(())
}
