use crate::{Index, Iter, NsIter, Payload};

impl<'a> Iter<'a> {
    pub fn new(block: &'a Payload) -> Self {
        Self {
            ns_iter: NsIter::new(&block.ns_table().len()).peekable(),
            tx_iter: None,
            block,
        }
    }
}

impl Iterator for Iter<'_> {
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some(ns_index) = self.ns_iter.peek() else {
                break None; // ns_iter consumed
            };

            if let Some(tx_index) = self
                .tx_iter
                .get_or_insert_with(|| self.block.ns_payload(ns_index).iter())
                .next()
            {
                break Some(Index {
                    namespace: ns_index.0 as u32,
                    position: tx_index.0 as u32,
                });
            }

            self.tx_iter = None; // unset `tx_iter`; it's consumed for this namespace
            self.ns_iter.next();
        }
    }
}
