// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HAHEN81's authored partition and native name-table file policy. The full
//! effectHahen.cpp/header and CNameList::LoadFile were inspected. Procedural
//! partition generation and polygon rasterization belong to shared workers.
use crate::config::{Names, integer};
use anyhow::{Result, ensure};
use std::{collections::BTreeMap, sync::Arc};

const VERTICES: [[i32; 2]; 66] = [
    [0, 0],
    [90, 0],
    [215, 0],
    [255, 0],
    [421, 0],
    [511, 0],
    [639, 0],
    [0, 70],
    [166, 65],
    [188, 33],
    [251, 50],
    [320, 102],
    [390, 72],
    [494, 56],
    [483, 86],
    [460, 58],
    [639, 87],
    [0, 136],
    [26, 121],
    [123, 175],
    [188, 146],
    [288, 169],
    [480, 177],
    [576, 155],
    [639, 197],
    [0, 188],
    [80, 242],
    [109, 249],
    [180, 278],
    [266, 272],
    [297, 224],
    [356, 265],
    [444, 235],
    [538, 286],
    [639, 249],
    [0, 259],
    [0, 309],
    [40, 343],
    [114, 338],
    [274, 330],
    [393, 321],
    [483, 347],
    [547, 331],
    [0, 385],
    [91, 411],
    [208, 402],
    [315, 381],
    [425, 379],
    [372, 408],
    [493, 435],
    [553, 377],
    [639, 429],
    [0, 479],
    [22, 479],
    [47, 450],
    [96, 456],
    [121, 445],
    [93, 479],
    [215, 479],
    [325, 466],
    [309, 479],
    [414, 479],
    [453, 479],
    [536, 479],
    [550, 479],
    [639, 479],
];
const FACES: [&[u32]; 32] = [
    &[0, 7, 18, 19, 20, 8, 1],
    &[1, 8, 2],
    &[2, 9, 10, 3],
    &[3, 10, 11, 12, 4],
    &[4, 12, 13, 5],
    &[5, 14, 22, 23, 16, 6],
    &[13, 15, 14],
    &[7, 17, 18],
    &[9, 8, 20, 21, 11, 10],
    &[15, 12, 11, 21, 30, 31, 32, 22, 14],
    &[16, 23, 24],
    &[18, 17, 25, 26, 27, 19],
    &[20, 19, 27, 28, 29, 30, 21],
    &[23, 22, 32, 33, 34, 24],
    &[25, 35, 26],
    &[26, 35, 36, 37, 38, 28, 27],
    &[29, 28, 38, 44, 56, 45, 39],
    &[30, 29, 39, 46, 47, 40, 31],
    &[32, 31, 40, 41, 42, 33],
    &[34, 33, 50, 51],
    &[36, 43, 37],
    &[38, 37, 43, 52, 53, 44],
    &[44, 54, 55, 56],
    &[54, 53, 57, 55],
    &[45, 55, 57, 58],
    &[39, 45, 58, 60, 59, 46],
    &[47, 46, 59, 48],
    &[48, 60, 61],
    &[40, 48, 61, 62, 49, 41],
    &[49, 62, 63],
    &[42, 41, 49, 63, 64, 50],
    &[50, 64, 65, 51],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Key(pub [i32; 5]);
impl Key {
    pub(crate) fn from_parameters(p: &[i32; 18]) -> Self {
        Self([p[13], p[14], p[15], p[16], p[17]])
    }
    pub(crate) fn filename(self) -> Option<Vec<u8>> {
        (self.0[0] == 2).then(|| format!("nya\\hahen{}.xtx", self.0[1]).into_bytes())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Partition {
    pub vertices: Vec<[i32; 2]>,
    pub faces: Vec<Vec<u32>>,
}
impl Partition {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            (1..=4096).contains(&self.vertices.len()),
            "NNN HAHEN vertex budget"
        );
        ensure!(
            (1..=4096).contains(&self.faces.len()),
            "NNN HAHEN polygon budget"
        );
        ensure!(
            self.vertices
                .iter()
                .flatten()
                .all(|v| v.unsigned_abs() <= 1_000_000),
            "NNN HAHEN coordinates"
        );
        ensure!(
            self.faces.iter().all(|f| (3..=15).contains(&f.len())
                && f.iter().all(|n| (*n as usize) < self.vertices.len())),
            "NNN HAHEN polygon indices"
        );
        Ok(())
    }
    pub(crate) fn default_for(canvas: [u32; 2]) -> Result<Self> {
        ensure!(
            canvas.iter().all(|v| (1..=16384).contains(v)),
            "NNN HAHEN canvas"
        );
        Ok(Self {
            vertices: VERTICES
                .map(|v| {
                    [
                        v[0].wrapping_mul(canvas[0] as i32)
                            .wrapping_div(640)
                            .min(canvas[0] as i32 - 1),
                        v[1].wrapping_mul(canvas[1] as i32)
                            .wrapping_div(480)
                            .min(canvas[1] as i32 - 1),
                    ]
                })
                .to_vec(),
            faces: FACES.iter().map(|f| f.to_vec()).collect(),
        })
    }

    /// `LoadFile(filename)` has angouFlag=FALSE even for this .xtx suffix.
    /// Missing/malformed tables are source-defined default geometry, not an
    /// unsupported operation acknowledged without rendering. I/O errors are
    /// kept distinct by the resource worker before this parser is called.
    pub(crate) fn from_external(bytes: &[u8], canvas: [u32; 2]) -> Result<Option<Self>> {
        let names = Names::parse(bytes, false)?;
        let numbers = names
            .tokens
            .iter()
            .map(|n| integer(n))
            .collect::<Result<Vec<_>>>()?;
        let Some(&n) = numbers.get(1) else {
            return Ok(None);
        };
        if n <= 0 {
            return Ok(None);
        }
        ensure!(n <= 4096, "NNN HAHEN external vertex budget");
        let n = n as usize;
        let Some(&count) = numbers.get(2 + n * 2) else {
            return Ok(None);
        };
        if count <= 0 {
            return Ok(None);
        }
        ensure!(count <= 4096, "NNN HAHEN external polygon budget");
        let vertices = numbers[2..2 + n * 2]
            .chunks_exact(2)
            .map(|p| [p[0], p[1]])
            .collect::<Vec<_>>();
        if vertices.iter().any(|v| {
            v.iter()
                .zip(canvas)
                .any(|(c, max)| *c < 0 || i64::from(*c) >= i64::from(max))
        }) {
            return Ok(None);
        }
        let mut at = 3 + n * 2;
        let mut faces = Vec::new();
        while let Some(&size) = numbers.get(at) {
            if !(3..=15).contains(&size) {
                break;
            }
            let end = at + 1 + size as usize;
            let Some(indices) = numbers.get(at + 1..end) else {
                break;
            };
            if indices.iter().any(|v| *v < 0 || *v as usize >= n) {
                break;
            }
            faces.push(indices.iter().map(|n| *n as u32).collect());
            if faces.len() > count as usize {
                break;
            }
            at = end;
        }
        // The native checker counts successfully read surfaces. It accepts a
        // malformed trailing token after the declared count, but a further
        // valid surface makes foundSurfaceKosuu too large and rejects it.
        if faces.len() != count as usize {
            return Ok(None);
        }
        let partition = Self { vertices, faces };
        partition.validate()?;
        Ok(Some(partition))
    }
}

#[derive(Clone)]
pub(crate) struct Partitions {
    canvas: [u32; 2],
    entries: BTreeMap<Key, Arc<Partition>>,
}

/// Retained CMyFile/NameList load for type 2. It uses the session's existing
/// capability-backed file view (also remote), never a local std::fs path or a
/// different image-pack search. A failed job is retained, not re-admitted on
/// every VM tick; dropping this owner cancels its Task.
pub(crate) struct ExternalPreparation {
    task: Option<crysirror_tasks::Task<Option<Partition>>>,
    ready: Option<Option<Partition>>,
    failed: Option<String>,
}
impl ExternalPreparation {
    pub(crate) fn begin(
        resources: &crate::resource_bridge::NativeResources,
        names: &crate::native_resources::ResourceNames,
        pool: &crysirror_tasks::TaskPool,
        key: Key,
        canvas: [u32; 2],
    ) -> Result<Self> {
        let filename = key
            .filename()
            .ok_or_else(|| anyhow::anyhow!("NNN HAHEN external partition type"))?;
        let path = names.encoded_path(&filename)?;
        let open = resources.file_view();
        ensure!(
            resources.client().0 != 0,
            "NNN HAHEN partition resource owner"
        );
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN HAHEN external canvas"
        );
        let task = pool.spawn(move |cancel| {
            cancel.check()?;
            let Some(reader) = open(&path, &cancel)? else {
                return Ok(None);
            };
            let bytes = crysirror_resource::read_all(reader.as_ref(), 1024 * 1024)?;
            cancel.check()?;
            let partition = Partition::from_external(&bytes, canvas)?;
            cancel.check()?;
            Ok(partition)
        })?;
        Ok(Self {
            task: Some(task),
            ready: None,
            failed: None,
        })
    }
    /// Outer None is pending; Some(None) is the native default fallback.
    pub(crate) fn poll(&mut self) -> Result<Option<Option<Partition>>> {
        if let Some(error) = &self.failed {
            anyhow::bail!("NNN HAHEN partition read: {error}")
        }
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let result = self
            .task
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("NNN HAHEN partition worker absent"))?
            .try_take();
        match result {
            Ok(Some(partition)) => {
                self.task = None;
                self.ready = Some(partition.clone());
                Ok(Some(partition))
            }
            Ok(None) => Ok(None),
            Err(error) => {
                self.failed = Some(format!("{error:#}"));
                Err(error)
            }
        }
    }
}
impl Partitions {
    pub(crate) fn new(canvas: [u32; 2]) -> Result<Self> {
        Ok(Self {
            canvas,
            entries: BTreeMap::from([(Key([0; 5]), Arc::new(Partition::default_for(canvas)?))]),
        })
    }
    pub(crate) fn get(&self, key: Key) -> Option<&Arc<Partition>> {
        self.entries.get(&key)
    }
    pub(crate) fn insert(
        &mut self,
        key: Key,
        generated: Option<Partition>,
    ) -> Result<Arc<Partition>> {
        if let Some(existing) = self.entries.get(&key) {
            return Ok(existing.clone());
        }
        ensure!(
            self.entries.len() < 1024,
            "NNN HAHEN partition cache budget"
        );
        let p = Arc::new(match generated {
            Some(p) => {
                p.validate()?;
                p
            }
            None => Partition::default_for(self.canvas)?,
        });
        self.entries.insert(key, p.clone());
        Ok(p)
    }
}
