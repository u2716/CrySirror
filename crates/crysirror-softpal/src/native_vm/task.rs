use crate::native_vm::animation::{
    PalSequenceAnimation, PalSequenceAnimationDesc, PalSheetAnimation, PalSheetAnimationDesc,
};
use crate::native_vm::input::PalInputState;
use crate::native_vm::sprite::SpriteSystem;
use std::collections::BTreeMap;

/// Portable task capacity. Native pools also reserve a separate child sentinel
/// per task, so their exhaustion threshold is not this many live tasks.
pub const TASK_POOL_CAPACITY: usize = 512;

/// Reviewed 32-bit PAL task storage. Pointer fields are guest address words,
/// never Rust pointers or portable TaskHandle values.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PalTaskNode32 {
    pub work: u32,
    pub work_size: u32,
    pub callback: u32,
    pub destructor: u32,
    pub message: u32,
    pub user_data: u32,
    pub flags: u32,
    pub state: u8,
    pub padding: [u8; 3],
    pub next: u32,
    pub previous: u32,
    pub child_sentinel: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PalWaitFrameData32 {
    pub remaining: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PalWaitTimeData32 {
    pub duration_ms: u32,
    pub start_ms: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PalWaitTimeExData32 {
    pub reserved: u32,
    pub duration_ms: u32,
    pub start_ms: u32,
}

const _: () = {
    assert!(std::mem::size_of::<PalTaskNode32>() == 44);
    assert!(std::mem::offset_of!(PalTaskNode32, state) == 28);
    assert!(std::mem::offset_of!(PalTaskNode32, next) == 32);
    assert!(std::mem::offset_of!(PalTaskNode32, child_sentinel) == 40);
    assert!(std::mem::size_of::<PalWaitFrameData32>() == 4);
    assert!(std::mem::size_of::<PalWaitTimeData32>() == 8);
    assert!(std::mem::size_of::<PalWaitTimeExData32>() == 12);
};

/// Blocking task flag (bit 16). When set on a task, normal processing of later siblings stops
/// after this task is processed. Only pending-free cleanup continues for remaining siblings.
/// PalWait and PalWaitTime use this flag.
pub const BLOCKING_FLAG: u32 = 0x10000;

fn debug_vm_wait_trace_enabled() -> bool {
    std::env::var("DEBUG_VM")
        .ok()
        .as_deref()
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

/// Handle to a task node. Encodes pool index in low 16 bits and generation counter
/// in high 16 bits so stale handles to reused slots are detected.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TaskHandle(pub u32);

/// Portable callback binding for an explicitly registered native callback word.
/// The handle translates the native work pointer into checked task-data access.
/// The u32 result represents raw EAX, not an inferred native C return type.
pub type TaskCallback = fn(&mut TaskSystem, &mut SpriteSystem, TaskHandle) -> u32;

impl TaskHandle {
    fn pool_index(self) -> usize {
        (self.0 & 0xFFFF) as usize
    }

    fn generation(self) -> u16 {
        (self.0 >> 16) as u16
    }

    fn encode(index: usize, generation: u16) -> Self {
        debug_assert!(index < TASK_POOL_CAPACITY);
        Self((generation as u32) << 16 | index as u32)
    }
}

/// Life-cycle state of a task node. Matches original PAL values 0..3.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum TaskState {
    #[default]
    Free = 0,
    Active = 1,
    /// Marked for release; release callback runs on the next process() call.
    PendingFree = 2,
    ChildSentinel = 3,
}

/// Known task payload types. Each variant mirrors the original PAL task data layout.
pub enum TaskKind {
    /// Slot is free; no payload.
    Free,
    /// Sheet (sprite-strip) animation. Corresponds to PalAnimation task data (0x1C bytes).
    AnimationSheet(PalSheetAnimation),
    /// Sequence (frame-record) animation. Corresponds to PalAnimationEx task data (0x18 bytes).
    AnimationSequence(PalSequenceAnimation),
    /// Frame-count wait. `remaining == -1` means wait forever.
    /// Corresponds to PalWait task data (4 bytes, remaining_count field).
    WaitFrame {
        remaining: i32,
        initialized: bool,
    },
    /// Time-based wait. Corresponds to PalWaitTime task data (8 bytes).
    WaitTime {
        duration_ms: u32,
        start_ms: u32,
        initialized: bool,
    },
    /// Explicit relative-task variant; work+0 is reserved, not the duration.
    WaitTimeEx {
        duration_ms: u32,
        start_ms: u32,
        initialized: bool,
    },
    /// Input-push wait. Frees itself when any key or mouse button is pushed.
    WaitClick,
    WaitSpriteFx {
        sprite: crate::native_vm::sprite::SpriteHandle,
    },
    /// Input-push wait with a timeout. Mirrors wait_click(duration >= 0).
    /// `voice_gated` marks the ADV auto-mode hold: the timeout only finishes
    /// the wait once voice playback has also ended (koikake.exe 0x42F9D0
    /// requires the PalSoundPlayIs voice check before advancing).
    WaitClickOrTime {
        duration_ms: u32,
        start_ms: u32,
        voice_gated: bool,
    },
    /// Generic PAL task node with modeled metadata but no Rust callback body yet.
    Raw,
    /// A task type not yet mapped to a Rust payload. Logs a warning each update and
    /// never frees itself. Callers must free it explicitly.
    Unsupported {
        name: &'static str,
    },
}

impl Default for TaskKind {
    fn default() -> Self {
        TaskKind::Free
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskUpdateOutcome {
    Continue,
    FreeSelf,
}

struct TaskNode {
    state: TaskState,
    state_byte: u8,
    kind: TaskKind,
    flags: u32,
    process: i32,
    end_process: i32,
    callback_override: bool,
    message: i32,
    user_data: u32,
    task_data: Vec<u8>,
    /// Incremented each time the slot is reused, to invalidate stale handles.
    generation: u16,
    /// Ordered list of child task pool indices.
    children: Vec<usize>,
    /// Index of parent task in pool. None = root list.
    parent: Option<usize>,
    name: &'static str,
}

impl Default for TaskNode {
    fn default() -> Self {
        Self {
            state: TaskState::Free,
            state_byte: 0,
            kind: TaskKind::Free,
            flags: 0,
            process: 0,
            end_process: 0,
            callback_override: false,
            message: 0,
            user_data: 0,
            task_data: Vec::new(),
            generation: 0,
            children: Vec::new(),
            parent: None,
            name: "",
        }
    }
}

/// PAL task scheduler. Owns a fixed pool of 512 task nodes and a root-level task list.
///
/// Processing is depth-first (children before siblings), matching sub_10264230.
/// Tasks with BLOCKING_FLAG stop normal sibling processing after they are handled;
/// only pending-free cleanup continues for remaining siblings.
pub struct TaskSystem {
    nodes: Vec<TaskNode>,
    root_children: Vec<usize>,
    /// PAL cached time in milliseconds. Updated once per engine frame by set_pal_time().
    pub pal_time_ms: u32,
    /// Next pool scan start index (circular scan for allocation).
    scan_start: usize,
    /// Native scheduler current task and independently selected work context.
    current_task: Option<TaskHandle>,
    task_data_context: Option<TaskHandle>,
    callbacks: BTreeMap<u32, TaskCallback>,
    callback_depth: u32,
}

impl TaskSystem {
    pub fn new() -> Self {
        let mut nodes = Vec::with_capacity(TASK_POOL_CAPACITY);
        for _ in 0..TASK_POOL_CAPACITY {
            nodes.push(TaskNode::default());
        }
        Self {
            nodes,
            root_children: Vec::new(),
            pal_time_ms: 0,
            scan_start: 0,
            current_task: None,
            task_data_context: None,
            callbacks: BTreeMap::new(),
            callback_depth: 0,
        }
    }

    /// Update the PAL cached time. Must be called once per frame before process().
    pub fn set_pal_time(&mut self, ms: u32) {
        self.pal_time_ms = ms;
    }

    /// True if the handle refers to a currently live (active or pending-free) task.
    pub fn is_alive(&self, handle: TaskHandle) -> bool {
        let idx = handle.pool_index();
        if idx >= TASK_POOL_CAPACITY {
            return false;
        }
        let node = &self.nodes[idx];
        node.state != TaskState::Free && node.generation == handle.generation()
    }

    fn is_present(&self, handle: TaskHandle) -> bool {
        self.nodes.get(handle.pool_index()).is_some_and(|node| {
            node.generation == handle.generation() && !matches!(node.kind, TaskKind::Free)
        })
    }

    /// Binding a callback word is explicit. It never casts or executes guest
    /// machine-code addresses. Zero retains the native null-callback meaning.
    pub fn register_callback(
        &mut self,
        word: u32,
        callback: TaskCallback,
    ) -> Result<(), &'static str> {
        if word == 0 {
            return Err("zero is the null PAL callback word");
        }
        self.callbacks.insert(word, callback);
        Ok(())
    }

    pub fn unregister_callback(&mut self, word: u32) {
        self.callbacks.remove(&word);
    }

    fn invoke_callback(
        &mut self,
        word: u32,
        task: TaskHandle,
        sprites: &mut SpriteSystem,
    ) -> Result<u32, &'static str> {
        let callback = self
            .callbacks
            .get(&word)
            .copied()
            .ok_or("unregistered PAL callback word")?;
        self.callback_depth += 1;
        let result = callback(self, sprites, task);
        self.callback_depth -= 1;
        Ok(result)
    }

    /// Do not let a polling callback perpetually replace an expired wait.
    pub fn wait_can_poll(&self, handle: TaskHandle, voice_active: bool) -> bool {
        if !self.is_alive(handle) {
            return false;
        }
        let node = &self.nodes[handle.pool_index()];
        if node.state != TaskState::Active {
            return false;
        }
        match node.kind {
            TaskKind::WaitClickOrTime {
                start_ms,
                duration_ms,
                voice_gated,
            } => {
                self.pal_time_ms.wrapping_sub(start_ms) < duration_ms || voice_gated && voice_active
            }
            _ => true,
        }
    }

    /// Preserve the absolute timer while a text callback runs in the VM.
    pub fn wait_clock(&self, handle: TaskHandle) -> Option<(u32, u32)> {
        if !self.is_alive(handle) {
            return None;
        }
        match self.nodes[handle.pool_index()].kind {
            TaskKind::WaitClickOrTime {
                start_ms,
                duration_ms,
                ..
            } => Some((start_ms, duration_ms)),
            _ => None,
        }
    }

    /// Allocate a free slot from the pool. Returns None if all 512 slots are occupied.
    fn alloc_node(&mut self) -> Option<usize> {
        let start = self.scan_start % TASK_POOL_CAPACITY;
        for offset in 0..TASK_POOL_CAPACITY {
            let idx = (start + offset) % TASK_POOL_CAPACITY;
            if self.nodes[idx].state == TaskState::Free
                && matches!(self.nodes[idx].kind, TaskKind::Free)
            {
                self.scan_start = (idx + 1) % TASK_POOL_CAPACITY;
                return Some(idx);
            }
        }
        None
    }

    fn insert_task(&mut self, idx: usize, flags: u32, parent: Option<usize>, name: &'static str) {
        self.nodes[idx].state = TaskState::Active;
        self.nodes[idx].state_byte = 1;
        self.nodes[idx].flags = flags;
        if self.nodes[idx].process == 0 {
            self.nodes[idx].process = 1;
        }
        self.nodes[idx].parent = parent;
        self.nodes[idx].name = name;
        self.nodes[idx].children.clear();
        match parent {
            None => self.root_children.push(idx),
            Some(p) => self.nodes[p].children.push(idx),
        }
    }

    /// Portable root/child-list constructor. For native flags and relative-task
    /// insertion use create_native_task instead.
    pub fn create_raw_task(
        &mut self,
        process: i32,
        end_process: i32,
        data_size: usize,
        flags: u32,
        parent: Option<TaskHandle>,
        name: &'static str,
    ) -> Option<TaskHandle> {
        if process == 0 {
            return None;
        }
        let idx = self.alloc_node()?;
        let parent_idx = parent.filter(|h| self.is_alive(*h)).map(|h| h.pool_index());
        let gen = self.nodes[idx].generation;
        self.nodes[idx].kind = TaskKind::Raw;
        self.nodes[idx].process = process;
        self.nodes[idx].end_process = end_process;
        self.nodes[idx].task_data = vec![0; data_size];
        self.nodes[idx].message = 0;
        self.insert_task(idx, flags, parent_idx, name);
        Some(TaskHandle::encode(idx, gen))
    }

    /// PalTaskCreate machine semantics with translated task pointers and
    /// registered callback words. Callback source return types remain unknown.
    /// Low flags word 1 inserts after relative_task; 2 prepends its child list;
    /// every other mode leaves the new task unlinked, as in the native body.
    pub fn create_native_task(
        &mut self,
        process: u32,
        destructor: u32,
        work_size: u32,
        flags: u32,
        relative_task: Option<TaskHandle>,
        user_data: u32,
    ) -> Option<TaskHandle> {
        if process == 0 {
            return None;
        }
        let relative = relative_task.or(self.current_task);
        let mode = flags as u16;
        let placement = if mode == 1 || mode == 2 {
            let relative = relative
                .filter(|handle| self.is_present(*handle))?
                .pool_index();
            if mode == 2 {
                Some((Some(relative), 0))
            } else {
                let parent = self.nodes[relative].parent;
                let position = self
                    .siblings(parent)
                    .iter()
                    .position(|idx| *idx == relative)?;
                Some((parent, position + 1))
            }
        } else {
            None
        };
        let mut work = Vec::new();
        work.try_reserve_exact(work_size as usize).ok()?;
        work.resize(work_size as usize, 0);
        let idx = self.alloc_node()?;
        let generation = self.nodes[idx].generation;
        let parent = placement.and_then(|(parent, _)| parent);
        self.nodes[idx].kind = TaskKind::Raw;
        self.nodes[idx].process = process as i32;
        self.nodes[idx].end_process = destructor as i32;
        self.nodes[idx].task_data = work;
        self.nodes[idx].user_data = user_data;
        self.insert_task(idx, flags, parent, "PalTask");
        self.siblings_mut(parent).pop();
        if let Some((parent, position)) = placement {
            self.siblings_mut(parent).insert(position, idx);
        }
        Some(TaskHandle::encode(idx, generation))
    }

    fn siblings(&self, parent: Option<usize>) -> &[usize] {
        match parent {
            Some(parent) => &self.nodes[parent].children,
            None => &self.root_children,
        }
    }

    fn siblings_mut(&mut self, parent: Option<usize>) -> &mut Vec<usize> {
        match parent {
            Some(parent) => &mut self.nodes[parent].children,
            None => &mut self.root_children,
        }
    }

    fn insert_wait_task(&mut self, idx: usize, name: &'static str) {
        let relative = (self.callback_depth != 0)
            .then_some(self.current_task)
            .flatten()
            .filter(|task| self.is_present(*task));
        if let Some(relative) = relative {
            let relative_idx = relative.pool_index();
            let parent = self.nodes[relative_idx].parent;
            if let Some(position) = self
                .siblings(parent)
                .iter()
                .position(|node| *node == relative_idx)
            {
                self.insert_task(idx, BLOCKING_FLAG | 1, parent, name);
                let siblings = self.siblings_mut(parent);
                siblings.pop();
                siblings.insert(position + 1, idx);
                return;
            }
        }
        self.insert_task(idx, BLOCKING_FLAG | 1, None, name);
    }

    /// Create a sheet animation task. `parent` = None means root list.
    pub fn create_animation_sheet(
        &mut self,
        sprites: &mut SpriteSystem,
        desc: PalSheetAnimationDesc,
        parent: Option<TaskHandle>,
    ) -> anyhow::Result<TaskHandle> {
        let idx = self.alloc_node().ok_or_else(|| {
            anyhow::anyhow!(
                "task pool full (capacity {}); cannot create sheet animation",
                TASK_POOL_CAPACITY
            )
        })?;
        let parent_idx = parent.filter(|h| self.is_alive(*h)).map(|h| h.pool_index());
        let anim = PalSheetAnimation::create(sprites, desc, self.pal_time_ms)?;
        let gen = self.nodes[idx].generation;
        self.nodes[idx].kind = TaskKind::AnimationSheet(anim);
        self.insert_task(idx, 0, parent_idx, "Animation");
        Ok(TaskHandle::encode(idx, gen))
    }

    /// Create a sequence animation task. `parent` = None means root list.
    pub fn create_animation_sequence(
        &mut self,
        sprites: &mut SpriteSystem,
        desc: PalSequenceAnimationDesc,
        parent: Option<TaskHandle>,
    ) -> anyhow::Result<TaskHandle> {
        let idx = self.alloc_node().ok_or_else(|| {
            anyhow::anyhow!(
                "task pool full (capacity {}); cannot create sequence animation",
                TASK_POOL_CAPACITY
            )
        })?;
        let parent_idx = parent.filter(|h| self.is_alive(*h)).map(|h| h.pool_index());
        let anim = PalSequenceAnimation::create(sprites, desc, self.pal_time_ms)?;
        let gen = self.nodes[idx].generation;
        self.nodes[idx].kind = TaskKind::AnimationSequence(anim);
        self.insert_task(idx, 0, parent_idx, "PalAnimationEx");
        Ok(TaskHandle::encode(idx, gen))
    }

    /// Create a PalWait blocking task. The first visit only initializes its
    /// callback. Later visits leave -1 alone and decrement every other value
    /// with 32-bit wrapping arithmetic; only an exact result of zero finishes.
    /// Returns None if the pool is full.
    pub fn create_wait_frame(&mut self, remaining: i32) -> Option<TaskHandle> {
        let idx = self.alloc_node()?;
        let gen = self.nodes[idx].generation;
        self.nodes[idx].kind = TaskKind::WaitFrame {
            remaining,
            initialized: false,
        };
        self.nodes[idx].task_data = remaining.to_le_bytes().to_vec();
        self.insert_wait_task(idx, "PalWait");
        let handle = TaskHandle::encode(idx, gen);
        if debug_vm_wait_trace_enabled() {
            log::debug!("[trace-wait] create_wait_frame handle={handle:?} remaining={remaining}");
        }
        Some(handle)
    }

    /// Create a PalWaitTime blocking task. Its first scheduled visit captures
    /// the cached PAL clock; creating a blocked task does not start its timer.
    /// Returns None if the pool is full.
    pub fn create_wait_time(&mut self, duration_ms: u32) -> Option<TaskHandle> {
        let idx = self.alloc_node()?;
        let gen = self.nodes[idx].generation;
        self.nodes[idx].kind = TaskKind::WaitTime {
            duration_ms,
            start_ms: 0,
            initialized: false,
        };
        let mut task_data = Vec::with_capacity(8);
        task_data.extend_from_slice(&duration_ms.to_le_bytes());
        task_data.extend_from_slice(&0_u32.to_le_bytes());
        self.nodes[idx].task_data = task_data;
        self.insert_wait_task(idx, "PalWaitTime");
        let handle = TaskHandle::encode(idx, gen);
        if debug_vm_wait_trace_enabled() {
            log::debug!(
                "[trace-wait] create_wait_time handle={handle:?} duration_ms={duration_ms}"
            );
        }
        Some(handle)
    }

    /// PalWaitTimeEx inserts immediately after the supplied relative task in
    /// the same sibling list (flags 0x10001), not into that task's children.
    /// None selects the scheduler's current task. Before any task is scheduled,
    /// the portable root insertion point substitutes for the native root task.
    /// An invalid explicit handle returns None instead of dereferencing it.
    pub fn create_wait_time_ex(
        &mut self,
        duration_ms: u32,
        relative_task: Option<TaskHandle>,
    ) -> Option<TaskHandle> {
        let relative_idx = match relative_task.or(self.current_task) {
            Some(handle) if self.is_alive(handle) => Some(handle.pool_index()),
            Some(_) => return None,
            None => None,
        };
        let parent = relative_idx.and_then(|idx| self.nodes[idx].parent);
        let insertion = relative_idx.and_then(|relative| {
            let siblings = match parent {
                Some(parent) => &self.nodes[parent].children,
                None => &self.root_children,
            };
            siblings
                .iter()
                .position(|idx| *idx == relative)
                .map(|pos| pos + 1)
        });
        if relative_idx.is_some() && insertion.is_none() {
            return None;
        }
        let idx = self.alloc_node()?;
        let generation = self.nodes[idx].generation;
        self.nodes[idx].kind = TaskKind::WaitTimeEx {
            duration_ms,
            start_ms: 0,
            initialized: false,
        };
        self.nodes[idx].task_data = vec![0; 12];
        self.nodes[idx].task_data[4..8].copy_from_slice(&duration_ms.to_le_bytes());
        self.insert_task(idx, BLOCKING_FLAG | 1, parent, "PalWaitTimeEx");
        if let Some(insertion) = insertion {
            let siblings = match parent {
                Some(parent) => &mut self.nodes[parent].children,
                None => &mut self.root_children,
            };
            siblings.pop();
            siblings.insert(insertion, idx);
        }
        Some(TaskHandle::encode(idx, generation))
    }

    pub fn create_wait_sprite_fx(
        &mut self,
        sprite: crate::native_vm::sprite::SpriteHandle,
    ) -> Option<TaskHandle> {
        let idx = self.alloc_node()?;
        let handle = TaskHandle::encode(idx, self.nodes[idx].generation);
        self.nodes[idx].kind = TaskKind::WaitSpriteFx { sprite };
        self.insert_task(idx, BLOCKING_FLAG, None, "PalWaitFx");
        Some(handle)
    }

    /// Create a WaitClick blocking task. Frees itself when any input push is detected.
    /// Returns None if the pool is full.
    pub fn create_wait_click(&mut self) -> Option<TaskHandle> {
        let idx = self.alloc_node()?;
        let gen = self.nodes[idx].generation;
        self.nodes[idx].kind = TaskKind::WaitClick;
        self.nodes[idx].task_data.clear();
        self.insert_task(idx, BLOCKING_FLAG, None, "PalWaitClick");
        let handle = TaskHandle::encode(idx, gen);
        log::debug!("[trace-wait] create_wait_click handle={handle:?}");
        Some(handle)
    }

    /// Create the PAL wait_click(duration) task: input ends it early, otherwise timeout ends it.
    /// Returns None if the pool is full.
    pub fn create_wait_click_or_time(&mut self, duration_ms: u32) -> Option<TaskHandle> {
        self.create_wait_click_or_time_gated(duration_ms, false)
    }

    /// `voice_gated` variant of `create_wait_click_or_time` used for ADV
    /// auto-mode holds: the timeout finishes the wait only after voice
    /// playback has ended as well.
    pub fn create_wait_click_or_time_gated(
        &mut self,
        duration_ms: u32,
        voice_gated: bool,
    ) -> Option<TaskHandle> {
        let idx = self.alloc_node()?;
        let gen = self.nodes[idx].generation;
        self.nodes[idx].kind = TaskKind::WaitClickOrTime {
            duration_ms: duration_ms.max(1),
            start_ms: self.pal_time_ms,
            voice_gated,
        };
        let mut task_data = Vec::with_capacity(8);
        task_data.extend_from_slice(&duration_ms.max(1).to_le_bytes());
        task_data.extend_from_slice(&self.pal_time_ms.to_le_bytes());
        self.nodes[idx].task_data = task_data;
        self.insert_task(idx, BLOCKING_FLAG, None, "PalWaitClickTime");
        let handle = TaskHandle::encode(idx, gen);
        log::debug!(
            "[trace-wait] create_wait_click_or_time handle={handle:?} duration_ms={} start_ms={}",
            duration_ms.max(1),
            self.pal_time_ms
        );
        Some(handle)
    }

    /// Mark a task as pending-free. All children are also marked pending-free.
    /// The task and its children will be released on the next process() call.
    /// Matches PalTaskFree_0 behavior (does not free immediately).
    pub fn free(&mut self, handle: TaskHandle) -> bool {
        if !self.is_alive(handle) {
            return false;
        }
        let idx = handle.pool_index();
        Self::mark_pending_free(&mut self.nodes, idx);
        true
    }

    fn mark_pending_free(nodes: &mut [TaskNode], idx: usize) {
        if nodes[idx].state == TaskState::Free {
            return;
        }
        nodes[idx].process = nodes[idx].end_process;
        nodes[idx].state = TaskState::PendingFree;
        nodes[idx].state_byte = 2;
        let children = nodes[idx].children.clone();
        for child in children {
            Self::mark_pending_free(nodes, child);
        }
    }

    /// Process all tasks depth-first. Animation callbacks update sprite source rects.
    /// Wait tasks check input state and free themselves when their condition is met.
    /// `skip_active` mirrors the native per-frame GetSkipState poll: wait-click
    /// style tasks complete immediately while any skip mode is active.
    /// `voice_active` gates ADV auto-mode hold timeouts on voice playback.
    pub fn process(
        &mut self,
        sprites: &mut SpriteSystem,
        input: &PalInputState,
        skip_active: bool,
        voice_active: bool,
    ) {
        self.process_list(None, 0, sprites, input, skip_active, voice_active);
    }

    /// Returns the number of live (active + pending-free) task slots.
    pub fn active_task_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.state != TaskState::Free)
            .count()
    }

    fn process_list(
        &mut self,
        parent: Option<usize>,
        start: usize,
        sprites: &mut SpriteSystem,
        input: &PalInputState,
        skip_active: bool,
        voice_active: bool,
    ) {
        let mut position = start;
        let mut pending_only = false;
        while let Some(&idx) = self.siblings(parent).get(position) {
            let task = TaskHandle::encode(idx, self.nodes[idx].generation);
            if !pending_only {
                // The live lists are retained during callbacks. Tasks inserted
                // after the current task can therefore run in this same pass.
                self.process_list(Some(idx), 0, sprites, input, skip_active, voice_active);
            }
            self.current_task = Some(task);
            if self.nodes[idx].state == TaskState::PendingFree {
                self.release_task(idx, sprites);
                // Native traversal resumes from the released node's previous
                // sibling, including its blocking flag, then follows next.
                if !pending_only && position != 0 {
                    if let Some(&previous) = self.siblings(parent).get(position - 1) {
                        pending_only = self.nodes[previous].flags & BLOCKING_FLAG != 0;
                    }
                }
                continue;
            }
            if !pending_only && self.nodes[idx].state == TaskState::Active {
                if matches!(self.nodes[idx].kind, TaskKind::Raw)
                    || self.nodes[idx].callback_override
                {
                    let process = self.nodes[idx].process as u32;
                    if process != 0 {
                        if let Err(reason) = self.invoke_callback(process, task, sprites) {
                            log::warn!(
                                "PAL task callback {process:#010x} was not dispatched: {reason}"
                            );
                        }
                    }
                } else {
                    let outcome = Self::do_update(
                        &mut self.nodes,
                        idx,
                        sprites,
                        input,
                        self.pal_time_ms,
                        skip_active,
                        voice_active,
                    );
                    if outcome == TaskUpdateOutcome::FreeSelf {
                        Self::mark_pending_free(&mut self.nodes, idx);
                    }
                }
            }
            // A reentrant callback may release this task. Do not dereference
            // its work or mistake a reused pool slot for the original task.
            if !self.is_present(task) {
                continue;
            }
            if !pending_only {
                pending_only = self.nodes[idx].flags & BLOCKING_FLAG != 0;
            }
            if let Some(current) = self.siblings(parent).iter().position(|node| *node == idx) {
                position = current + 1;
            } else {
                break;
            }
        }
    }

    fn release_task(&mut self, idx: usize, sprites: &mut SpriteSystem) {
        let task = TaskHandle::encode(idx, self.nodes[idx].generation);
        let process = self.nodes[idx].process as u32;
        let parent = self.nodes[idx].parent;
        self.nodes[idx].state = TaskState::Free;
        self.nodes[idx].state_byte = 0;
        // State is already zero, but work remains available to the destructor.
        // ChangeState(2) without Free invokes the current process, not a
        // separately substituted destructor; dispatch follows that exact word.
        if process != 0
            && (matches!(self.nodes[idx].kind, TaskKind::Raw) || self.nodes[idx].callback_override)
        {
            if let Err(reason) = self.invoke_callback(process, task, sprites) {
                log::warn!(
                    "PAL task cleanup callback {process:#010x} was not dispatched: {reason}"
                );
            }
        }
        if let Some(position) = self.siblings(parent).iter().position(|node| *node == idx) {
            self.siblings_mut(parent).remove(position);
        }
        let generation = self.nodes[idx].generation.wrapping_add(1);
        self.nodes[idx] = TaskNode {
            generation,
            ..TaskNode::default()
        };
        // The native pending-only branch clears the child sentinel without
        // walking descendants. Preserve that observed orphaning behavior.
    }

    fn do_update(
        nodes: &mut Vec<TaskNode>,
        idx: usize,
        sprites: &mut SpriteSystem,
        input: &PalInputState,
        pal_time_ms: u32,
        skip_active: bool,
        voice_active: bool,
    ) -> TaskUpdateOutcome {
        if nodes[idx].process == 0 {
            return TaskUpdateOutcome::Continue;
        }
        // Take the kind out to avoid aliasing with the nodes slice during the update.
        let mut kind = std::mem::replace(&mut nodes[idx].kind, TaskKind::Free);
        let outcome = match &mut kind {
            TaskKind::AnimationSheet(anim) => anim.step_once(sprites, pal_time_ms),
            TaskKind::AnimationSequence(anim) => anim.step_once(sprites, pal_time_ms),
            TaskKind::WaitFrame {
                remaining,
                initialized,
            } => {
                if !*initialized {
                    *initialized = true;
                    TaskUpdateOutcome::Continue
                } else {
                    let data = &mut nodes[idx].task_data;
                    if let Some(bytes) = data.get_mut(..4) {
                        *remaining = i32::from_le_bytes(bytes.try_into().unwrap());
                        if *remaining != -1 {
                            *remaining = remaining.wrapping_sub(1);
                            bytes.copy_from_slice(&remaining.to_le_bytes());
                        }
                    }
                    if *remaining == 0 && data.len() >= 4 {
                        TaskUpdateOutcome::FreeSelf
                    } else {
                        TaskUpdateOutcome::Continue
                    }
                }
            }
            TaskKind::WaitTime {
                duration_ms,
                start_ms,
                initialized,
            } => Self::update_time_wait(
                &mut nodes[idx].task_data,
                0,
                duration_ms,
                start_ms,
                initialized,
                pal_time_ms,
            ),
            TaskKind::WaitTimeEx {
                duration_ms,
                start_ms,
                initialized,
            } => Self::update_time_wait(
                &mut nodes[idx].task_data,
                4,
                duration_ms,
                start_ms,
                initialized,
                pal_time_ms,
            ),
            TaskKind::WaitSpriteFx { sprite } => {
                if input.click_push() || skip_active || sprites.sprite_fx_effect_state(*sprite) == 0
                {
                    TaskUpdateOutcome::FreeSelf
                } else {
                    TaskUpdateOutcome::Continue
                }
            }
            TaskKind::WaitClick => {
                // Native wait-click consumers poll GetSkipState every frame:
                // any latched skip mode (skip byte, scene skip, held Ctrl)
                // completes the wait just like a click push. Only the native
                // click set (Space/Return/left mouse/wheel up) qualifies;
                // Escape, arrows and F-keys must not advance text.
                if input.click_push() || skip_active {
                    log::debug!(
                        "[trace-wait] wait_click complete idx={idx} click_push={} skip_active={skip_active} pal_time_ms={pal_time_ms}",
                        input.click_push()
                    );
                    TaskUpdateOutcome::FreeSelf
                } else {
                    TaskUpdateOutcome::Continue
                }
            }
            TaskKind::WaitClickOrTime {
                duration_ms,
                start_ms,
                voice_gated,
            } => {
                let elapsed = pal_time_ms.wrapping_sub(*start_ms);
                // Auto-mode holds (voice_gated) finish by timeout only once
                // voice playback has ended; skip state and input always
                // complete immediately (koikake.exe 0x42F9D0/0x4326DF).
                let timed_out = elapsed >= *duration_ms && !(*voice_gated && voice_active);
                if input.click_push() || skip_active || timed_out {
                    log::debug!(
                        "[trace-wait] wait_click_or_time complete idx={idx} click_push={} skip_active={skip_active} elapsed={elapsed} duration_ms={duration_ms} voice_gated={voice_gated} voice_active={voice_active}",
                        input.click_push()
                    );
                    TaskUpdateOutcome::FreeSelf
                } else {
                    TaskUpdateOutcome::Continue
                }
            }
            TaskKind::Unsupported { name } => {
                log::warn!(
                    "TaskSystem: unsupported task '{}' encountered; it will never self-complete",
                    name
                );
                TaskUpdateOutcome::Continue
            }
            TaskKind::Raw => TaskUpdateOutcome::Continue,
            TaskKind::Free => TaskUpdateOutcome::FreeSelf,
        };
        nodes[idx].kind = kind;
        outcome
    }

    fn update_time_wait(
        data: &mut [u8],
        offset: usize,
        duration_ms: &mut u32,
        start_ms: &mut u32,
        initialized: &mut bool,
        pal_time_ms: u32,
    ) -> TaskUpdateOutcome {
        let Some(data) = data.get_mut(offset..offset + 8) else {
            // Replacing a work block with an undersized block is a native
            // invalid-memory access; the portable adapter leaves it inert.
            return TaskUpdateOutcome::Continue;
        };
        *duration_ms = u32::from_le_bytes(data[..4].try_into().unwrap());
        if !*initialized {
            *initialized = true;
            *start_ms = pal_time_ms;
            data[4..8].copy_from_slice(&start_ms.to_le_bytes());
            return TaskUpdateOutcome::Continue;
        }
        *start_ms = u32::from_le_bytes(data[4..8].try_into().unwrap());
        if pal_time_ms.wrapping_sub(*start_ms) >= *duration_ms {
            TaskUpdateOutcome::FreeSelf
        } else {
            TaskUpdateOutcome::Continue
        }
    }

    // ---- Animation accessors ----

    /// Mark an animation task as pending-free.
    pub fn animation_release(&mut self, handle: TaskHandle) -> bool {
        self.free(handle)
    }

    /// Reset a sheet animation to frame 0 and update last_time_ms.
    pub fn animation_reset(&mut self, handle: TaskHandle, sprites: &mut SpriteSystem) -> bool {
        if !self.is_alive(handle) {
            return false;
        }
        let idx = handle.pool_index();
        let t = self.pal_time_ms;
        match &mut self.nodes[idx].kind {
            TaskKind::AnimationSheet(anim) => anim.reset(sprites, t).is_ok(),
            TaskKind::AnimationSequence(anim) => anim.reset(sprites, t).is_ok(),
            _ => false,
        }
    }

    /// Reset a sequence animation with a new frame table and restart it.
    pub fn animation_reset_sequence_with_frames(
        &mut self,
        handle: TaskHandle,
        sprites: &mut SpriteSystem,
        frames: Vec<crate::native_vm::animation::PalAnimationFrameRecord>,
    ) -> bool {
        if !self.is_alive(handle) {
            return false;
        }
        let idx = handle.pool_index();
        let t = self.pal_time_ms;
        match &mut self.nodes[idx].kind {
            TaskKind::AnimationSequence(anim) => anim.reset_with_frames(sprites, frames, t).is_ok(),
            _ => false,
        }
    }

    pub fn animation_start(&mut self, handle: TaskHandle) -> bool {
        if !self.is_alive(handle) {
            return false;
        }
        let idx = handle.pool_index();
        match &mut self.nodes[idx].kind {
            TaskKind::AnimationSheet(anim) => {
                anim.running = true;
                true
            }
            TaskKind::AnimationSequence(anim) => {
                anim.running = true;
                true
            }
            _ => false,
        }
    }

    pub fn animation_stop(&mut self, handle: TaskHandle) -> bool {
        if !self.is_alive(handle) {
            return false;
        }
        let idx = handle.pool_index();
        match &mut self.nodes[idx].kind {
            TaskKind::AnimationSheet(anim) => {
                anim.running = false;
                true
            }
            TaskKind::AnimationSequence(anim) => {
                anim.running = false;
                true
            }
            _ => false,
        }
    }

    /// Set frame delay and reset last_time_ms. Only valid for sheet animations.
    pub fn animation_set_time(&mut self, handle: TaskHandle, frame_delay_ms: u32) -> bool {
        if !self.is_alive(handle) {
            return false;
        }
        let idx = handle.pool_index();
        let t = self.pal_time_ms;
        match &mut self.nodes[idx].kind {
            TaskKind::AnimationSheet(anim) => {
                anim.frame_delay_ms = frame_delay_ms;
                anim.last_time_ms = t;
                true
            }
            _ => false,
        }
    }

    /// Returns frame_delay_ms for a sheet animation handle.
    /// For sequence handles, returns None (original PalAnimationGetTime_0 reads task_data+0x04
    /// which is the frame-record table pointer for Ex, not a time value).
    pub fn animation_get_time(&self, handle: TaskHandle) -> Option<u32> {
        if !self.is_alive(handle) {
            return None;
        }
        match &self.nodes[handle.pool_index()].kind {
            TaskKind::AnimationSheet(anim) => Some(anim.frame_delay_ms),
            _ => None,
        }
    }

    /// Returns the raw value at task_data+0x10.
    /// For Ex (sequence), this is current_frame. For sheet, this is the flags byte (as dword).
    pub fn animation_get_point(&self, handle: TaskHandle) -> Option<i32> {
        if !self.is_alive(handle) {
            return None;
        }
        match &self.nodes[handle.pool_index()].kind {
            TaskKind::AnimationSheet(anim) => Some(i32::from(anim.flags.raw())),
            TaskKind::AnimationSequence(anim) => Some(anim.current_frame),
            _ => None,
        }
    }

    /// Consume and return the looped flag for a sheet animation (one-shot read).
    pub fn animation_take_looped(&mut self, handle: TaskHandle) -> bool {
        if !self.is_alive(handle) {
            return false;
        }
        let idx = handle.pool_index();
        match &mut self.nodes[idx].kind {
            TaskKind::AnimationSheet(anim) => std::mem::take(&mut anim.looped_since_last_query),
            _ => false,
        }
    }

    pub fn task_state(&self, handle: TaskHandle) -> Option<TaskState> {
        if !self.is_present(handle) {
            return None;
        }
        let node = &self.nodes[handle.pool_index()];
        if node.process == node.end_process {
            Some(TaskState::PendingFree)
        } else {
            Some(node.state)
        }
    }

    pub fn task_state_raw(&self, handle: TaskHandle) -> i32 {
        if !self.is_present(handle) {
            return 0;
        }
        let node = &self.nodes[handle.pool_index()];
        if node.process == node.end_process {
            2
        } else {
            i32::from(node.state_byte)
        }
    }

    /// Native null task arguments select the scheduler current task. These
    /// methods consume translated handles, not native pointer words; the ABI
    /// bridge must perform address translation before calling them.
    pub fn native_free(&mut self, task: Option<TaskHandle>) -> i32 {
        i32::from(
            task.or(self.current_task)
                .is_some_and(|task| self.free(task)),
        )
    }

    pub fn native_get_message(&mut self, task: Option<TaskHandle>) -> Option<u32> {
        let task = task.or(self.current_task)?;
        self.is_present(task)
            .then(|| self.take_message(task) as u32)
    }

    pub fn native_get_state(&self, task: Option<TaskHandle>) -> Option<u32> {
        let task = task.or(self.current_task)?;
        self.is_present(task)
            .then(|| self.task_state_raw(task) as u32)
    }

    /// Semantic adapter only: the native source return is unresolved. The
    /// Result reports portable address-validation failure, not native EAX.
    pub fn native_set_message(
        &mut self,
        task: Option<TaskHandle>,
        message: u32,
    ) -> Result<(), &'static str> {
        let task = task.or(self.current_task).ok_or("no current PAL task")?;
        if self.set_message(task, message as i32) {
            Ok(())
        } else {
            Err("invalid PAL task")
        }
    }

    /// PalTaskSetTaskData selects a task, without changing either work block.
    /// Its native source return remains unresolved.
    pub fn native_select_task_data(
        &mut self,
        task: Option<TaskHandle>,
    ) -> Result<(), &'static str> {
        let task = task.or(self.current_task).ok_or("no current PAL task")?;
        if !self.is_present(task) {
            return Err("invalid PAL task");
        }
        self.task_data_context = Some(task);
        Ok(())
    }

    pub fn native_get_task_data(&self, task: Option<TaskHandle>) -> Option<&[u8]> {
        self.task_data(task.or(self.task_data_context)?)
    }

    pub fn native_get_task_data_mut(&mut self, task: Option<TaskHandle>) -> Option<&mut [u8]> {
        self.task_data_mut(task.or(self.task_data_context)?)
    }

    /// Runs the selected task and its later siblings, then restores the saved
    /// scheduler context. This is PalTaskReProcess control flow; the portable
    /// Result is not a declaration of its unresolved native source return.
    pub fn reprocess(
        &mut self,
        task: Option<TaskHandle>,
        sprites: &mut SpriteSystem,
        input: &PalInputState,
        skip_active: bool,
        voice_active: bool,
    ) -> Result<(), &'static str> {
        let task = task.or(self.current_task).ok_or("no current PAL task")?;
        if !self.is_present(task) {
            return Err("invalid PAL task");
        }
        let idx = task.pool_index();
        let parent = self.nodes[idx].parent;
        let siblings = match parent {
            Some(parent) => &self.nodes[parent].children,
            None => &self.root_children,
        };
        let start = siblings
            .iter()
            .position(|sibling| *sibling == idx)
            .ok_or("unlinked PAL task")?;
        let saved_current = self.current_task;
        self.process_list(parent, start, sprites, input, skip_active, voice_active);
        self.current_task = saved_current;
        Ok(())
    }

    pub fn change_state(&mut self, handle: TaskHandle, state: TaskState) -> bool {
        if !self.is_present(handle) {
            return false;
        }
        self.nodes[handle.pool_index()].state = state;
        self.nodes[handle.pool_index()].state_byte = state as u8;
        true
    }

    pub fn set_message(&mut self, handle: TaskHandle, message: i32) -> bool {
        if !self.is_present(handle) {
            return false;
        }
        self.nodes[handle.pool_index()].message = message;
        true
    }

    pub fn take_message(&mut self, handle: TaskHandle) -> i32 {
        if !self.is_present(handle) {
            return 0;
        }
        std::mem::take(&mut self.nodes[handle.pool_index()].message)
    }

    pub fn task_data(&self, handle: TaskHandle) -> Option<&[u8]> {
        if !self.is_present(handle) {
            return None;
        }
        Some(&self.nodes[handle.pool_index()].task_data)
    }

    pub fn task_data_mut(&mut self, handle: TaskHandle) -> Option<&mut [u8]> {
        if !self.is_present(handle) {
            return None;
        }
        Some(&mut self.nodes[handle.pool_index()].task_data)
    }

    /// Portable work replacement utility, not PalTaskSetTaskData. Native code
    /// selects a task-data context with native_select_task_data instead.
    pub fn set_task_data(&mut self, handle: TaskHandle, data: Vec<u8>) -> bool {
        if !self.is_present(handle) {
            return false;
        }
        self.nodes[handle.pool_index()].task_data = data;
        true
    }

    pub fn change_next(&mut self, handle: TaskHandle, process: i32) -> bool {
        if !self.is_present(handle) {
            return false;
        }
        self.nodes[handle.pool_index()].process = process;
        self.nodes[handle.pool_index()].callback_override = true;
        true
    }

    /// PalTaskChangeNext semantic adapter. Result is portable validation only;
    /// it does not assert the unresolved native source return contract.
    pub fn native_change_next(&mut self, process: u32) -> Result<(), &'static str> {
        let Some(task) = self.current_task else {
            return Ok(());
        };
        if self.change_next(task, process as i32) {
            Ok(())
        } else {
            Err("invalid current PAL task")
        }
    }

    /// Installs the callback before invoking it immediately, forwarding its
    /// raw 32-bit result. This never executes an unregistered guest address.
    pub fn native_change_next_immediate(
        &mut self,
        process: u32,
        sprites: &mut SpriteSystem,
    ) -> Result<u32, &'static str> {
        let task = self.current_task.ok_or("no current PAL task")?;
        if !self.change_next(task, process as i32) {
            return Err("invalid current PAL task");
        }
        self.invoke_callback(process, task, sprites)
    }

    /// Stores the low state byte, including unnamed values. Only state 1 runs
    /// normal callbacks and state 2 runs deferred cleanup. Other bytes retain
    /// their raw value and still participate in child traversal and blocking.
    pub fn native_change_state(
        &mut self,
        task: Option<TaskHandle>,
        state_word: u32,
    ) -> Result<(), &'static str> {
        let task = task.or(self.current_task).ok_or("no current PAL task")?;
        if !self.is_present(task) {
            return Err("invalid PAL task");
        }
        let state_byte = state_word as u8;
        let node = &mut self.nodes[task.pool_index()];
        node.state_byte = state_byte;
        node.state = match state_byte {
            0 => TaskState::Free,
            1 => TaskState::Active,
            2 => TaskState::PendingFree,
            _ => TaskState::ChildSentinel,
        };
        Ok(())
    }

    pub fn task_user_data(&self, task: TaskHandle) -> Option<u32> {
        self.is_present(task)
            .then(|| self.nodes[task.pool_index()].user_data)
    }

    pub fn get_animation_sheet(&self, handle: TaskHandle) -> Option<&PalSheetAnimation> {
        if !self.is_alive(handle) {
            return None;
        }
        match &self.nodes[handle.pool_index()].kind {
            TaskKind::AnimationSheet(anim) => Some(anim),
            _ => None,
        }
    }

    pub fn get_animation_sequence(&self, handle: TaskHandle) -> Option<&PalSequenceAnimation> {
        if !self.is_alive(handle) {
            return None;
        }
        match &self.nodes[handle.pool_index()].kind {
            TaskKind::AnimationSequence(anim) => Some(anim),
            _ => None,
        }
    }

    /// Returns the count of all non-free task slots.
    pub fn active_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.state != TaskState::Free)
            .count()
    }

    /// Returns a dump of all active (non-free) task slots for diagnostic output.
    pub fn dump_entries(&self) -> Vec<TaskDumpEntry> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.state != TaskState::Free)
            .map(|(idx, n)| TaskDumpEntry {
                handle: TaskHandle::encode(idx, n.generation),
                kind_name: n.name,
                state: n.state,
                flags: n.flags,
                parent_index: n.parent,
                children_count: n.children.len(),
                is_blocking: (n.flags & BLOCKING_FLAG) != 0,
                is_pending_free: n.state == TaskState::PendingFree,
            })
            .collect()
    }

    /// Returns whether any task in the pool has BLOCKING_FLAG set and is Active.
    pub fn has_blocking_task(&self) -> bool {
        self.nodes
            .iter()
            .any(|n| n.state == TaskState::Active && (n.flags & BLOCKING_FLAG) != 0)
    }
}

impl Default for TaskSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct TaskDumpEntry {
    pub handle: TaskHandle,
    pub kind_name: &'static str,
    pub state: TaskState,
    pub flags: u32,
    pub parent_index: Option<usize>,
    pub children_count: usize,
    pub is_blocking: bool,
    pub is_pending_free: bool,
}

/// Type alias: animation handles are task handles.
pub type AnimationHandle = TaskHandle;
