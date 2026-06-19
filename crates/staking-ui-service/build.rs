use vergen::Emitter;
use vergen_gitcl::GitclBuilder;

pub fn main() -> anyhow::Result<()> {
    let git = GitclBuilder::default()
        .sha(false)
        .describe(true, true, None)
        .commit_timestamp(true)
        .build()?;
    Emitter::default().add_instructions(&git)?.emit()?;
    Ok(())
}
