use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PalListHandle(pub u32);

#[derive(Clone, Debug, Eq, PartialEq)]
struct PalListNode {
    data: i32,
    tag: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct PalList {
    nodes: Vec<PalListNode>,
}

#[derive(Clone, Debug, Default)]
pub struct PalListSystem {
    next_handle: u32,
    lists: BTreeMap<PalListHandle, PalList>,
}

impl PalListSystem {
    pub fn new() -> Self {
        Self {
            next_handle: 1,
            lists: BTreeMap::new(),
        }
    }

    pub fn create(&mut self) -> PalListHandle {
        let handle = PalListHandle(self.next_handle);
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .expect("PAL list handle space exhausted");
        self.lists.insert(handle, PalList::default());
        handle
    }

    pub fn release(&mut self, handle: PalListHandle) -> bool {
        self.lists.remove(&handle).is_some()
    }

    pub fn push(&mut self, handle: PalListHandle, data: i32, tag: i32) -> bool {
        let Some(list) = self.lists.get_mut(&handle) else {
            return false;
        };
        list.nodes.insert(0, PalListNode { data, tag });
        true
    }

    pub fn push_last(&mut self, handle: PalListHandle, data: i32, tag: i32) -> bool {
        let Some(list) = self.lists.get_mut(&handle) else {
            return false;
        };
        list.nodes.push(PalListNode { data, tag });
        true
    }

    pub fn delete_data(&mut self, handle: PalListHandle, data: i32) -> bool {
        let Some(list) = self.lists.get_mut(&handle) else {
            return false;
        };
        let Some(index) = list.nodes.iter().position(|node| node.data == data) else {
            return false;
        };
        list.nodes.remove(index);
        true
    }

    pub fn pop(&mut self, handle: PalListHandle, tag: i32) -> i32 {
        let Some(list) = self.lists.get_mut(&handle) else {
            return 0;
        };
        let Some(index) = list.nodes.iter().position(|node| node.tag == tag) else {
            return 0;
        };
        list.nodes.remove(index).data
    }

    pub fn pop_first(&mut self, handle: PalListHandle) -> i32 {
        let Some(list) = self.lists.get_mut(&handle) else {
            return 0;
        };
        if list.nodes.is_empty() {
            0
        } else {
            list.nodes.remove(0).data
        }
    }

    pub fn find(&self, handle: PalListHandle, tag: i32) -> Option<usize> {
        self.lists
            .get(&handle)?
            .nodes
            .iter()
            .position(|node| node.tag == tag)
    }

    pub fn find_next(&self, handle: PalListHandle, cursor: usize) -> Option<usize> {
        let list = self.lists.get(&handle)?;
        let tag = list.nodes.get(cursor)?.tag;
        list.nodes
            .iter()
            .enumerate()
            .skip(cursor.saturating_add(1))
            .find_map(|(index, node)| (node.tag == tag).then_some(index))
    }

    pub fn get_data(&self, handle: PalListHandle, tag: i32, ordinal: usize) -> i32 {
        let Some(list) = self.lists.get(&handle) else {
            return 0;
        };
        list.nodes
            .iter()
            .filter(|node| node.tag == tag)
            .nth(ordinal)
            .map(|node| node.data)
            .unwrap_or(0)
    }

    pub fn get_data_count(&self, handle: PalListHandle, tag: i32) -> usize {
        self.lists
            .get(&handle)
            .map(|list| list.nodes.iter().filter(|node| node.tag == tag).count())
            .unwrap_or(0)
    }

    pub fn len(&self, handle: PalListHandle) -> usize {
        self.lists
            .get(&handle)
            .map(|list| list.nodes.len())
            .unwrap_or(0)
    }
}

impl Default for PalListHandle {
    fn default() -> Self {
        Self(0)
    }
}
