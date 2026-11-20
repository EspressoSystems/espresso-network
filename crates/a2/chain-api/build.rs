fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .compile_protos(&["proto/espresso/a2/v1/api.proto"], &["proto"])?;
    Ok(())
}
