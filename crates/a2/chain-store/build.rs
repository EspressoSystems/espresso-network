fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        // `Bytes` instead of `Vec<u8>`, so payloads pass through without a copy.
        .bytes(".")
        .compile_protos(&["proto/espresso/a2/store/v1/store.proto"], &["proto"])?;
    Ok(())
}
