//! # Reward Merkle Tree v2 Persistence Benchmark
//!
//! This benchmark helps determine the optimal way to persist the reward merkle tree.
//!
//! ## Problem
//! The reward merkle tree v2 is a sparse binary tree (height 160) that stores reward account
//! balances. We need to decide how to persist this data structure to disk for:
//! - Fast startup/recovery (minimize time to get a working tree in memory)
//! - Reasonable storage space
//! - Ability to share snapshots between nodes
//!
//! ## Options Tested
//! 1. **Key/Value pairs (bincode)**: Extract (account, balance) pairs and serialize
//!    - Pro: Minimal storage (only leaves)
//!    - Con: Must rebuild entire tree on load (~800ms for 10k accounts)
//!
//! 2. **Full tree (bincode)**: Serialize the entire tree structure with all intermediate nodes
//!    - Pro: Fastest load time (no rebuild needed)
//!    - Con: 171x larger storage (includes all Merkle proof nodes)
//!
//! 3. **Full tree (gzip)**: Compress the serialized tree
//!    - Pro: Good balance - 2x compression, fast load
//!    - Con: Slower write (compression overhead)
//!
//! ## Key Metrics
//! - Storage size (bytes per account)
//! - Write time (memory → disk)
//! - Read time (disk → memory with usable tree)

use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
    time::Instant,
};

use alloy::primitives::{Address, U256};
use clap::Parser;
use espresso_types::{
    v0_3::RewardAmount,
    v0_4::{RewardAccountV2, RewardMerkleTreeV2, REWARD_MERKLE_TREE_V2_HEIGHT},
};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use jf_merkle_tree_compat::{MerkleTreeScheme, UniversalMerkleTreeScheme};
use rand::{rngs::StdRng, Rng, SeedableRng};

/// RAII guard to ensure temp directory cleanup
struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(path: PathBuf) -> anyhow::Result<Self> {
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[derive(Parser, Debug)]
#[command(about = "Test Reward Merkle Tree v2 serialization and compression")]
struct Args {
    /// Number of accounts to create in the tree
    #[arg(short, long, default_value = "10000")]
    num_accounts: usize,

    /// Random seed for reproducibility
    #[arg(short, long, default_value = "42")]
    seed: u64,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let num_accounts = args.num_accounts;

    const MIN_VALUE: u128 = 1_000_000_000_000_000_000; // 1 * 1e18
    const MAX_VALUE: u128 = 10_000_000_000_000_000_000_000; // 10k * 1e18

    println!(
        "Creating Reward Merkle Tree v2 with {} accounts...",
        num_accounts
    );
    println!("Height: {}", REWARD_MERKLE_TREE_V2_HEIGHT);
    println!("Seed: {}", args.seed);
    println!();

    // Create temp directory for test data
    let temp_dir = PathBuf::from("./tmp/reward-mt-test");
    let _temp_guard = TempDirGuard::new(temp_dir.clone())?;
    let accounts_file = temp_dir.join("accounts.bin");

    // Generate random accounts and values and write to disk
    println!("Generating random accounts and values...");
    let mut rng = StdRng::seed_from_u64(args.seed);
    let mut accounts = Vec::with_capacity(num_accounts);

    for _i in 0..num_accounts {
        // Random 20-byte address
        let mut addr_bytes = [0u8; 20];
        rng.fill(&mut addr_bytes);
        let account = RewardAccountV2(Address::from_slice(&addr_bytes));

        // Random value in range [1e18, 10k * 1e18]
        let value = rng.gen_range(MIN_VALUE..=MAX_VALUE);
        let amount = RewardAmount(U256::from(value));

        accounts.push((account, amount));
    }
    println!("Generated {} random accounts", accounts.len());

    // Write accounts to disk
    println!("Writing accounts to disk...");
    let write_start = Instant::now();
    let file = File::create(&accounts_file)?;
    let mut writer = BufWriter::new(file);
    bincode::serialize_into(&mut writer, &accounts)?;
    writer.flush()?;
    let write_duration = write_start.elapsed();
    println!("Wrote accounts to disk in {:?}", write_duration);

    let file_size = fs::metadata(&accounts_file)?.len();
    println!(
        "Accounts file size: {:.2} MB",
        file_size as f64 / 1024.0 / 1024.0
    );
    println!();

    // Read accounts back from disk to simulate loading data
    println!("Reading accounts from disk...");
    let read_start = Instant::now();
    let file = File::open(&accounts_file)?;
    let reader = BufReader::new(file);
    let accounts: Vec<(RewardAccountV2, RewardAmount)> = bincode::deserialize_from(reader)?;
    let read_duration = read_start.elapsed();
    println!(
        "Read {} accounts from disk in {:?}",
        accounts.len(),
        read_duration
    );
    println!();

    // Create tree and populate (time only the insertion)
    let mut tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

    println!("Populating tree with accounts...");
    let start = Instant::now();

    for (i, (account, amount)) in accounts.iter().enumerate() {
        tree.update(*account, *amount)?;

        if (i + 1) % 1000 == 0 {
            println!("  Inserted {} accounts...", i + 1);
        }
    }

    let populate_duration = start.elapsed();
    println!("Tree population took: {:?}", populate_duration);
    println!(
        "  Average per insert: {:?}",
        populate_duration / u32::try_from(num_accounts).unwrap()
    );
    println!();

    // Get commitment to verify tree is populated
    let commitment = tree.commitment();
    println!("Tree commitment: {:?}", commitment);
    println!();

    // Extract all key/values from the tree
    println!("Extracting all key/values from tree...");
    let extract_start = Instant::now();
    let entries: Vec<(RewardAccountV2, RewardAmount)> =
        tree.iter().map(|(k, v)| (*k, *v)).collect();
    let extract_duration = extract_start.elapsed();
    println!(
        "  Extracted {} entries in {:?}",
        entries.len(),
        extract_duration
    );

    // Serialize key/values
    let serialize_entries_start = Instant::now();
    let serialized_entries = bincode::serialize(&entries)?;
    let serialize_entries_duration = serialize_entries_start.elapsed();

    // Write key/values to disk
    let entries_file = temp_dir.join("entries.bin");
    let write_entries_start = Instant::now();
    let file = File::create(&entries_file)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(&serialized_entries)?;
    writer.flush()?;
    let write_entries_duration = write_entries_start.elapsed();

    let entries_size = serialized_entries.len();
    let entries_mb = entries_size as f64 / 1024.0 / 1024.0;

    println!(
        "  Serialize entries (memory): {:?}",
        serialize_entries_duration
    );
    println!("  Write entries to disk: {:?}", write_entries_duration);
    println!(
        "  Total (extract + serialize + write): {:?}",
        extract_duration + serialize_entries_duration + write_entries_duration
    );
    println!(
        "  Entries size: {:.2} MB ({} bytes)",
        entries_mb, entries_size
    );
    println!(
        "  Bytes per entry: {:.0}",
        entries_size as f64 / entries.len() as f64
    );

    // Test reading entries back from disk
    println!("Reading entries from disk...");
    let read_entries_start = Instant::now();
    let file = File::open(&entries_file)?;
    let mut reader = BufReader::new(file);
    let mut read_entries_data = Vec::new();
    reader.read_to_end(&mut read_entries_data)?;
    let read_entries_duration = read_entries_start.elapsed();

    let deserialize_entries_start = Instant::now();
    let _deserialized_entries: Vec<(RewardAccountV2, RewardAmount)> =
        bincode::deserialize(&read_entries_data)?;
    let deserialize_entries_duration = deserialize_entries_start.elapsed();

    println!("  Read from disk: {:?}", read_entries_duration);
    println!("  Deserialize (memory): {:?}", deserialize_entries_duration);
    println!(
        "  Total (read + deserialize): {:?}",
        read_entries_duration + deserialize_entries_duration
    );
    println!();

    // Compress entries with gzip level 1
    println!("Compressing entries with gzip level 1...");
    let compress_entries_start = Instant::now();
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&serialized_entries)?;
    let compressed_entries = encoder.finish()?;
    let compress_entries_duration = compress_entries_start.elapsed();

    let compressed_entries_file = temp_dir.join("entries.bin.gz");
    let write_compressed_entries_start = Instant::now();
    let file = File::create(&compressed_entries_file)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(&compressed_entries)?;
    writer.flush()?;
    let write_compressed_entries_duration = write_compressed_entries_start.elapsed();

    let compressed_entries_size = compressed_entries.len();
    let compressed_entries_mb = compressed_entries_size as f64 / 1024.0 / 1024.0;
    let entries_compression_ratio = entries_size as f64 / compressed_entries_size as f64;

    println!("  Compression (memory): {:?}", compress_entries_duration);
    println!("  Write to disk: {:?}", write_compressed_entries_duration);
    println!(
        "  Total (compress + write): {:?}",
        compress_entries_duration + write_compressed_entries_duration
    );
    println!(
        "  Compressed size: {:.2} MB ({} bytes)",
        compressed_entries_mb, compressed_entries_size
    );
    println!("  Compression ratio: {:.2}x", entries_compression_ratio);
    println!();

    // Test reading compressed entries back from disk
    println!("Reading compressed entries from disk...");
    let read_compressed_entries_start = Instant::now();
    let file = File::open(&compressed_entries_file)?;
    let mut reader = BufReader::new(file);
    let mut compressed_entries_data = Vec::new();
    reader.read_to_end(&mut compressed_entries_data)?;
    let read_compressed_entries_duration = read_compressed_entries_start.elapsed();

    let decompress_entries_start = Instant::now();
    let mut decoder = GzDecoder::new(&compressed_entries_data[..]);
    let mut decompressed_entries_data = Vec::new();
    decoder.read_to_end(&mut decompressed_entries_data)?;
    let decompress_entries_duration = decompress_entries_start.elapsed();

    let deserialize_decompressed_entries_start = Instant::now();
    let _deserialized_entries: Vec<(RewardAccountV2, RewardAmount)> =
        bincode::deserialize(&decompressed_entries_data)?;
    let deserialize_decompressed_entries_duration =
        deserialize_decompressed_entries_start.elapsed();

    println!("  Read from disk: {:?}", read_compressed_entries_duration);
    println!("  Decompress (memory): {:?}", decompress_entries_duration);
    println!(
        "  Deserialize (memory): {:?}",
        deserialize_decompressed_entries_duration
    );
    println!(
        "  Total (read + decompress + deserialize): {:?}",
        read_compressed_entries_duration
            + decompress_entries_duration
            + deserialize_decompressed_entries_duration
    );
    println!();

    // Test serialization in memory
    println!("Serializing tree with bincode (in memory)...");
    let serialize_mem_start = Instant::now();
    let serialized = bincode::serialize(&tree)?;
    let serialize_mem_duration = serialize_mem_start.elapsed();

    let size_bytes = serialized.len();
    let size_kb = size_bytes as f64 / 1024.0;
    let size_mb = size_kb / 1024.0;

    println!("  Serialization (memory): {:?}", serialize_mem_duration);
    println!(
        "  Serialized size: {} bytes ({:.2} KB, {:.2} MB)",
        size_bytes, size_kb, size_mb
    );
    println!();

    // Write serialized data to disk
    let tree_file = temp_dir.join("tree.bin");
    println!("Writing serialized tree to disk...");
    let write_disk_start = Instant::now();
    let file = File::create(&tree_file)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(&serialized)?;
    writer.flush()?;
    let write_disk_duration = write_disk_start.elapsed();
    println!("  Disk write: {:?}", write_disk_duration);
    println!(
        "  Total (serialize + write): {:?}",
        serialize_mem_duration + write_disk_duration
    );
    println!();

    // Read serialized data from disk
    println!("Reading serialized tree from disk...");
    let read_disk_start = Instant::now();
    let file = File::open(&tree_file)?;
    let mut reader = BufReader::new(file);
    let mut serialized_from_disk = Vec::new();
    reader.read_to_end(&mut serialized_from_disk)?;
    let read_disk_duration = read_disk_start.elapsed();
    println!("  Disk read: {:?}", read_disk_duration);

    // Test deserialization from memory
    println!("Deserializing tree (from memory)...");
    let deserialize_mem_start = Instant::now();
    let deserialized_tree: RewardMerkleTreeV2 = bincode::deserialize(&serialized_from_disk)?;
    let deserialize_mem_duration = deserialize_mem_start.elapsed();
    println!("  Deserialization (memory): {:?}", deserialize_mem_duration);
    println!(
        "  Total (read + deserialize): {:?}",
        read_disk_duration + deserialize_mem_duration
    );
    println!();

    // Verify commitment matches
    let deserialized_commitment = deserialized_tree.commitment();
    println!(
        "Deserialized tree commitment: {:?}",
        deserialized_commitment
    );

    if commitment == deserialized_commitment {
        println!("✓ Commitments match!");
    } else {
        println!("✗ Commitments DO NOT match!");
        return Err(anyhow::anyhow!("Commitment mismatch after deserialization"));
    }
    println!();

    // Test compression with different levels
    println!("Testing compression with different gzip levels...");
    println!();

    let levels = vec![(Compression::fast(), "fast (level 1)")];

    let mut first_compressed_file = PathBuf::new();
    let mut compress_duration = std::time::Duration::ZERO;
    let mut write_time = std::time::Duration::ZERO;
    let mut compressed_size = 0;
    let mut compressed_mb = 0.0;
    let mut compression_ratio = 0.0;

    for (compression, label) in levels {
        println!("Gzip {}:", label);

        // Compress in memory (reuse serialized from line 247)
        let compress_mem_start = Instant::now();
        let mut encoder = GzEncoder::new(Vec::new(), compression);
        encoder.write_all(&serialized)?;
        let compressed = encoder.finish()?;
        let compress_mem_duration = compress_mem_start.elapsed();

        // Write compressed data to disk
        let compressed_file = temp_dir.join(format!("tree-{}.bin.gz", label.replace(' ', "-")));
        let write_start = Instant::now();
        let file = File::create(&compressed_file)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&compressed)?;
        writer.flush()?;
        let write_time_local = write_start.elapsed();

        let compressed_size_local = compressed.len();
        let compressed_mb_local = compressed_size_local as f64 / 1024.0 / 1024.0;
        let compression_ratio_local = size_bytes as f64 / compressed_size_local as f64;

        // Save for summary table
        compress_duration = compress_mem_duration;
        write_time = write_time_local;
        compressed_size = compressed_size_local;
        compressed_mb = compressed_mb_local;
        compression_ratio = compression_ratio_local;

        println!("  Compression (memory): {:?}", compress_mem_duration);
        println!("  Write to disk: {:?}", write_time_local);
        println!(
            "  Total (compress + write): {:?}",
            compress_mem_duration + write_time_local
        );
        println!(
            "  Compressed size: {:.2} MB ({} bytes)",
            compressed_mb_local, compressed_size_local
        );
        println!("  Compression ratio: {:.2}x", compression_ratio_local);
        println!(
            "  Compressed bytes per account: {:.0}",
            compressed_size_local as f64 / num_accounts as f64
        );

        // Save the first compression for decompression test
        if first_compressed_file.as_os_str().is_empty() {
            first_compressed_file = compressed_file;
        }

        println!();
    }

    // Test decompression from disk with level 1 (fastest)
    println!("Testing full tree decompression...");

    // Read compressed file from disk
    let read_compressed_tree_start = Instant::now();
    let file = File::open(&first_compressed_file)?;
    let mut reader = BufReader::new(file);
    let mut compressed_tree_data = Vec::new();
    reader.read_to_end(&mut compressed_tree_data)?;
    let read_compressed_tree_duration = read_compressed_tree_start.elapsed();

    // Decompress in memory
    let decompress_tree_start = Instant::now();
    let mut decoder = GzDecoder::new(&compressed_tree_data[..]);
    let mut decompressed_tree_data = Vec::new();
    decoder.read_to_end(&mut decompressed_tree_data)?;
    let decompress_tree_duration = decompress_tree_start.elapsed();

    // Deserialize tree
    let deserialize_decompressed_tree_start = Instant::now();
    let _tree_from_compressed: RewardMerkleTreeV2 = bincode::deserialize(&decompressed_tree_data)?;
    let deserialize_decompressed_tree_duration = deserialize_decompressed_tree_start.elapsed();

    println!(
        "  Read compressed from disk: {:?}",
        read_compressed_tree_duration
    );
    println!("  Decompression (memory): {:?}", decompress_tree_duration);
    println!(
        "  Deserialization (memory): {:?}",
        deserialize_decompressed_tree_duration
    );
    println!(
        "  Total (read + decompress + deserialize): {:?}",
        read_compressed_tree_duration
            + decompress_tree_duration
            + deserialize_decompressed_tree_duration
    );
    println!();

    // Verify decompressed data matches original serialized tree
    if serialized_from_disk == decompressed_tree_data {
        println!("✓ Decompressed data matches!");
    } else {
        println!("✗ Decompressed data does NOT match!");
        return Err(anyhow::anyhow!("Data mismatch after decompression"));
    }
    println!();

    // Summary
    println!("=== SUMMARY ===");
    println!("Accounts: {}", num_accounts);
    println!("Tree height: {}", REWARD_MERKLE_TREE_V2_HEIGHT);
    println!();
    println!("Account generation:");
    println!("  Write to disk: {:?}", write_duration);
    println!("  Read from disk: {:?}", read_duration);
    println!();
    println!("Tree population: {:?}", populate_duration);
    println!();

    // Comparison table
    println!("=== STORAGE COMPARISON ===");
    println!();
    println!(
        "{:<35} {:>12} {:>20} {:>20} {:>10}",
        "Method", "Size (MB)", "Memory → Disk", "Disk → Memory", "Ratio"
    );
    println!("{}", "-".repeat(100));

    // KV pairs (need to include tree population on read)
    let kv_write_time = extract_duration + serialize_entries_duration + write_entries_duration;
    let kv_read_time = read_entries_duration + deserialize_entries_duration + populate_duration;
    println!(
        "{:<35} {:>12.2} {:>20} {:>20} {:>10.2}x",
        "Key/Value pairs (bincode)",
        entries_mb,
        format!("{:.1}ms", kv_write_time.as_secs_f64() * 1000.0),
        format!("{:.1}ms", kv_read_time.as_secs_f64() * 1000.0),
        1.0
    );

    // Full tree
    let tree_write_time = serialize_mem_duration + write_disk_duration;
    let tree_read_time = read_disk_duration + deserialize_mem_duration;
    println!(
        "{:<35} {:>12.2} {:>20} {:>20} {:>10.2}x",
        "Full tree (bincode)",
        size_mb,
        format!("{:.1}ms", tree_write_time.as_secs_f64() * 1000.0),
        format!("{:.1}ms", tree_read_time.as_secs_f64() * 1000.0),
        size_bytes as f64 / entries_size as f64
    );

    // Full tree compressed
    let tree_gz_write_time = serialize_mem_duration + compress_duration + write_time;
    let tree_gz_read_time = read_compressed_tree_duration
        + decompress_tree_duration
        + deserialize_decompressed_tree_duration;
    println!(
        "{:<35} {:>12.2} {:>20} {:>20} {:>10.2}x",
        "Full tree (gzip level 1)",
        compressed_mb,
        format!("{:.1}ms", tree_gz_write_time.as_secs_f64() * 1000.0),
        format!("{:.1}ms", tree_gz_read_time.as_secs_f64() * 1000.0),
        compressed_size as f64 / entries_size as f64
    );

    println!();
    println!("Note: Memory → Disk includes all processing (extract/serialize/compress + write)");
    println!(
        "      Disk → Memory includes all processing (read + decompress/deserialize + tree \
         rebuild for KV)"
    );
    println!("      Size ratio is relative to key/value pairs size");
    println!();

    println!("Bytes per account:");
    println!(
        "  Key/Value pairs:     {:.0} bytes",
        entries_size as f64 / num_accounts as f64
    );
    println!(
        "  Full tree:           {:.0} bytes ({:.1}x)",
        size_bytes as f64 / num_accounts as f64,
        size_bytes as f64 / entries_size as f64
    );
    println!(
        "  Full tree (gzip):    {:.0} bytes ({:.2}x)",
        compressed_size as f64 / num_accounts as f64,
        compression_ratio
    );

    // Cleanup handled by TempDirGuard on drop
    println!();
    println!("Done!");

    Ok(())
}
