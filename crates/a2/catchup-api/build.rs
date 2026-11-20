fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .compile_protos(&["proto/espresso/a2/catchup/v1/catchup.proto"], &["proto"])?;
    Ok(())
}
