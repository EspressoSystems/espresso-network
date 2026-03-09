#[cfg(not(feature = "refactored"))]
mod legacy;

#[cfg(feature = "refactored")]
mod refactored;

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "refactored")]
    {
        refactored::main()
    }

    #[cfg(not(feature = "refactored"))]
    {
        legacy::main()
    }
}
