use vergen::{BuildBuilder, CargoBuilder, Emitter};

pub fn main() -> anyhow::Result<()> {
    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let cargo = CargoBuilder::default()
        .debug(true)
        .features(true)
        .target_triple(true)
        .build()?;
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .emit()?;
    Ok(())
}
