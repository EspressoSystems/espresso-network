use vergen::Emitter;
use vergen_gitcl::GitclBuilder;

pub fn main() -> anyhow::Result<()> {
    let git = GitclBuilder::default()
        .sha(false) // false = full SHA, not short
        .describe(true, true, None)
        .dirty(true)
        .branch(true)
        .commit_timestamp(true)
        .build()?;
    Emitter::default().add_instructions(&git)?.emit()?;
    Ok(())
}
