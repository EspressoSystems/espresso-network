use vergen::{BuildBuilder, CargoBuilder, Emitter};
use vergen_gitcl::GitclBuilder;

pub fn main() -> anyhow::Result<()> {
    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let cargo = CargoBuilder::default()
        .debug(true)
        .features(true)
        .target_triple(true)
        .build()?;
    let git = GitclBuilder::default()
        .sha(false)
        .describe(true, true, None)
        .dirty(true)
        .branch(true)
        .commit_timestamp(true)
        .build()?;
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&git)?
        .emit()?;
    Ok(())
}
