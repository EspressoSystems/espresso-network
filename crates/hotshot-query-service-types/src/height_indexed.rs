/// Types which have a notion of "height" within a chain.
pub trait HeightIndexed {
    fn height(&self) -> u64;
}

impl<T: HeightIndexed, U> HeightIndexed for (T, U) {
    fn height(&self) -> u64 {
        self.0.height()
    }
}

#[cfg(feature = "testing")]
mod testing {
    use hotshot_example_types::block_types::TestBlockHeader;

    use super::*;

    impl HeightIndexed for TestBlockHeader {
        fn height(&self) -> u64 {
            self.block_number
        }
    }
}
