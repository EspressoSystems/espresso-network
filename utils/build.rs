use vergen::{BuildBuilder, Emitter};
use vergen_gitcl::GitclBuilder;

pub fn main() -> anyhow::Result<()> {
    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let git = GitclBuilder::default()
        .sha(false) // false = full SHA, not short
        .describe(true, true, None)
        .dirty(true)
        .branch(true)
        .commit_timestamp(true)
        .build()?;
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&git)?
        .emit()?;
    Ok(())
}
