use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PalMemoryHandle(pub u32);

#[derive(Clone, Debug, Default)]
pub struct PalMemorySystem {
    next_handle: u32,
    allocations: BTreeMap<PalMemoryHandle, Vec<u8>>,
}

impl PalMemorySystem {
    pub fn new() -> Self {
        Self {
            next_handle: 1,
            allocations: BTreeMap::new(),
        }
    }

    /// PalMemoryAlloc(size, flags): allocate bytes and zero them when flags bit 0 is set.
    pub fn alloc(&mut self, size: usize, flags: u8) -> Option<PalMemoryHandle> {
        if size == 0 {
            return None;
        }
        let handle = PalMemoryHandle(self.next_handle);
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .expect("PAL memory handle space exhausted");
        let mut bytes = vec![0u8; size];
        if flags & 1 == 0 {
            bytes.fill(0xCD);
        }
        self.allocations.insert(handle, bytes);
        Some(handle)
    }

    /// PalMemoryFree(ptr): ignores null/unknown handles and frees known allocations.
    pub fn free(&mut self, handle: PalMemoryHandle) -> bool {
        self.allocations.remove(&handle).is_some()
    }

    pub fn get(&self, handle: PalMemoryHandle) -> Option<&[u8]> {
        self.allocations.get(&handle).map(Vec::as_slice)
    }

    pub fn get_mut(&mut self, handle: PalMemoryHandle) -> Option<&mut [u8]> {
        self.allocations.get_mut(&handle).map(Vec::as_mut_slice)
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }

    pub fn allocated_bytes(&self) -> usize {
        self.allocations.values().map(Vec::len).sum()
    }
}
