#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let _profiler = dhat::Profiler::new_heap();

    sequencer::main().await
}
