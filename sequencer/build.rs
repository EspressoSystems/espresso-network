use vergen::{CargoBuilder, Emitter};

pub fn main() -> anyhow::Result<()> {
    let cargo = CargoBuilder::default().features(true).build()?;
    Emitter::default().add_instructions(&cargo)?.emit()?;
    Ok(())
}
