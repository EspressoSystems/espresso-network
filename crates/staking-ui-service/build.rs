use vergen::EmitBuilder;

pub fn main() -> anyhow::Result<()> {
    // Set an environment variable with git information
    EmitBuilder::builder()
        .git_sha(false)
        .git_describe(true, true, None)
        .git_commit_timestamp()
        .emit()?;
    Ok(())
}
