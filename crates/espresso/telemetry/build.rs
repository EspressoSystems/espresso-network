fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/remote.proto");
    println!("cargo:rerun-if-changed=proto/types.proto");
    prost_build::Config::new()
        .compile_protos(&["proto/remote.proto", "proto/types.proto"], &["proto/"])?;
    Ok(())
}
