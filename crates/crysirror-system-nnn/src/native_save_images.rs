// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native SetEffectFileNameByLoad policy over the same engine-owned image
//! preparation used by ordinary LOADDWQ. No synthetic VM instruction is made.
use crate::{
    image_bridge::{PicturePreparation, PreparedPicture},
    native_bootstrap::NativeConfiguration,
    native_save::{LegacySaveProfile, NativeSaveFile, SavedEffect},
    resource_bridge::NativeResources,
};
use anyhow::{ensure, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_tasks::TaskPool;

pub(crate) struct SavedPictures {
    client: ClientId,
    effects: Vec<SavedEffect>,
    requests: Vec<(i32, Vec<u8>)>,
    next: usize,
    active: Option<PicturePreparation>,
    prepared: Vec<PreparedPicture>,
    names: crate::native_resources::ResourceNames,
    levels: u32,
    resources: NativeResources,
    pool: TaskPool,
    failure: Option<String>,
    pub api_calls: u64,
}
impl SavedPictures {
    /// Validate the entire original record/name set before resource admission.
    /// Original flag0 or empty filenames deliberately preserve warm slots.
    pub fn new(
        client: ClientId,
        file: &NativeSaveFile,
        profile: &LegacySaveProfile,
        configuration: &NativeConfiguration,
        resources: NativeResources,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0 && resources.client() == client,
            "NNN saved pictures capability scope"
        );
        file.validate(profile)?;
        let effects = file.effect_records()?;
        let levels = u32::try_from(configuration.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
        ensure!(levels <= 1024, "NNN saved picture pack profile");
        let mut requests = Vec::new();
        for (slot, effect) in effects.iter().enumerate() {
            if effect.enabled() && !effect.filename.is_empty() {
                configuration
                    .resources
                    .script_image(&effect.filename, levels)?;
                requests.push((slot as i32, effect.filename.clone()));
            }
        }
        Ok(Self {
            client,
            effects,
            requests,
            next: 0,
            active: None,
            prepared: vec![],
            names: configuration.resources.clone(),
            levels,
            resources,
            pool: TaskPool::new("nnn-native-saved-pictures", 1, 1)?,
            failure: None,
            api_calls: 0,
        })
    }
    pub fn poll(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if let Some(reason) = &self.failure {
            anyhow::bail!("NNN saved picture preparation: {reason}");
        }
        let result = self.advance(host);
        if let Err(reason) = &result {
            self.failure = Some(format!("{reason:#}"));
        }
        result
    }
    fn advance(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if self.next == self.requests.len() {
            return Ok(true);
        }
        if self.active.is_none() {
            let (slot, name) = &self.requests[self.next];
            self.active = Some(PicturePreparation::begin(
                self.client,
                *slot,
                name.clone(),
                false,
                &self.names,
                self.levels,
                self.resources.clone(),
                &self.pool,
            )?);
        }
        let active = self.active.as_mut().unwrap();
        let before = active.api_calls;
        let result = active.poll(host);
        self.api_calls += active.api_calls - before;
        if let Some(picture) = result? {
            self.prepared.push(picture);
            self.active = None;
            self.next += 1;
        }
        Ok(self.next == self.requests.len())
    }
    pub fn effects(&self) -> &[SavedEffect] {
        &self.effects
    }
    pub fn into_prepared(self) -> Result<Vec<PreparedPicture>> {
        ensure!(
            self.failure.is_none() && self.next == self.requests.len() && self.active.is_none(),
            "NNN saved pictures are not completely prepared"
        );
        // The caller overlays these prepared slots only after all native state
        // families validate; inactive/unnamed slots are not silently cleared.
        Ok(self.prepared)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Names, expression::VariableProfile, native_resources::ResourceNames,
        native_save::SaveBlock, XorProfile,
    };
    use crysirror_core::service::MemoryAssetReader;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{Duration, Instant},
    };
    fn fixture() -> (NativeSaveFile, LegacySaveProfile, NativeConfiguration) {
        let profile = LegacySaveProfile {
            header_name: b"image-fixture".to_vec(),
            version: 1,
            extension_sizes: vec![],
            omake: false,
            cutin: false,
        };
        let mut info = vec![0; 32];
        info[..4].copy_from_slice(&8u32.to_le_bytes());
        info[4..8].copy_from_slice(&1u32.to_le_bytes());
        let mut mini = vec![0; 20];
        mini[..4].copy_from_slice(&1u32.to_le_bytes());
        mini[4..8].copy_from_slice(&1u32.to_le_bytes());
        let file = NativeSaveFile {
            blocks: vec![
                SaveBlock::new(0, b"image-fixture", vec![]).unwrap(),
                SaveBlock::new(1, b"INFO", info).unwrap(),
                SaveBlock::new(2, b"GAMESTATUS", vec![0; 7056]).unwrap(),
                SaveBlock::new(3, b"MINICG", mini).unwrap(),
                SaveBlock::new(4, b"GAMEVAR", vec![0; 4096]).unwrap(),
                SaveBlock::new(5, b"EFFECT", vec![0; 16384]).unwrap(),
                SaveBlock::new(6, b"EFFECTFILENAME", vec![0; 4096]).unwrap(),
                SaveBlock::new(7, b"MESSAGE", vec![0; 16384]).unwrap(),
            ],
        };
        let empty = Names::parse(b"", false).unwrap();
        let cfg = NativeConfiguration {
            resources: ResourceNames::new("").unwrap(),
            setup: empty.clone(),
            main_control: None,
            variable_names: empty.clone(),
            project: empty.clone(),
            commands: empty.clone(),
            functions: empty,
            initial: None,
            module_sha256: "authored-reviewed-native-layout".into(),
            profile: VariableProfile::Legacy1000,
            scripts: XorProfile::default(),
        };
        (file, profile, cfg)
    }
    fn name(file: &mut NativeSaveFile, slot: usize, value: &[u8]) {
        let at = slot * 256;
        file.blocks[6].payload[at..at + value.len()].copy_from_slice(value);
    }
    fn word(file: &mut NativeSaveFile, slot: usize, index: usize, value: i32) {
        let at = slot * 1024 + index * 4;
        file.blocks[5].payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn runtime() -> (tempfile::TempDir, crysirror_runtime::Runtime) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("game")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"native saved images",
            "content_revision":"1","assets":"game","canvas":[16,16],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        crysirror_formats_blackcyc::register_images(&mut runtime.assets.decoders).unwrap();
        runtime.tick(0, Default::default()).unwrap();
        (dir, runtime)
    }
    #[test]
    fn saved_flagged_pictures_use_real_shared_jobs_without_vm_receipts_or_visible_state_changes() {
        let (mut file, profile, cfg) = fixture();
        for slot in [0, 4] {
            word(&mut file, slot, 0, 1);
            name(&mut file, slot, b"bgcue");
        }
        name(&mut file, 1, b"inactive");
        word(&mut file, 2, 0, 1); // Original inactive/unnamed slots do not reload.
        for index in 3..27 {
            word(&mut file, 0, index, index as i32);
        }
        word(&mut file, 0, 27, -3);
        word(&mut file, 0, 28, 72);
        word(&mut file, 0, 29, 0x12345678);
        let before_bytes = file.encode(&profile).unwrap();
        let mut dwq = vec![b' '; 64];
        dwq[..4].copy_from_slice(b"PNG ");
        dwq[36..40].copy_from_slice(&2u32.to_le_bytes());
        dwq[40..44].copy_from_slice(&1u32.to_le_bytes());
        dwq[48..64].copy_from_slice(b"PACKTYPE=8      ");
        dwq.extend(
            crysirror_image::encode_png(&crysirror_image::ImageSurface {
                width: 2,
                height: 1,
                rgba: vec![4, 8, 12, 31, 16, 20, 24, 255],
            })
            .unwrap(),
        );
        let bytes: Arc<[u8]> = dwq.into();
        let reads = Arc::new(AtomicUsize::new(0));
        let hits = reads.clone();
        let resources = NativeResources::blackcyc(
            ClientId(1),
            Arc::new(move |path, cancel| {
                cancel.check()?;
                assert_eq!(path, "dwq/bg/bgcue.dwq");
                hits.fetch_add(1, Ordering::SeqCst);
                Ok(Some(Arc::new(MemoryAssetReader(bytes.clone()))))
            }),
        )
        .unwrap();
        let (_dir, mut runtime) = runtime();
        let scene = serde_json::to_value(&runtime.host.engine).unwrap();
        let mut prepared =
            SavedPictures::new(ClientId(1), &file, &profile, &cfg, resources).unwrap();
        assert_eq!(reads.load(Ordering::SeqCst), 0, "construction is dormant");
        assert_eq!(prepared.effects()[0].rectangles().source, [3, 4, 5, 6]);
        assert_eq!(
            prepared.effects()[0].rectangles().destination_from,
            [15, 16, 17, 18]
        );
        assert_eq!(prepared.effects()[0].words[29], 0x12345678);
        let end = Instant::now() + Duration::from_secs(10);
        while !prepared.poll(&mut runtime.host).unwrap() {
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
        assert!(
            prepared.poll(&mut runtime.host).unwrap(),
            "Ready is retained"
        );
        assert_eq!(reads.load(Ordering::SeqCst), 2);
        let pictures = prepared.into_prepared().unwrap();
        assert_eq!(pictures.len(), 2);
        assert_eq!((pictures[0].slot, pictures[1].slot), (0, 4));
        assert!(pictures.iter().all(|picture| picture.has_mask));
        assert_eq!(
            pictures[0].raw.rgba().as_ref(),
            [4, 8, 12, 31, 16, 20, 24, 255]
        );
        assert_eq!(
            pictures[0].display.rgba().as_ref(),
            [4, 8, 12, 255, 16, 20, 24, 255]
        );
        assert_eq!(
            pictures[1].display.rgba().as_ref(),
            [4, 8, 12, 31, 16, 20, 24, 255]
        );
        assert_eq!(runtime.host.now_ms(), 0);
        assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), scene);
        assert_eq!(file.encode(&profile).unwrap(), before_bytes);
    }
    #[test]
    fn corrupt_names_and_foreign_scope_fail_before_io_and_missing_resource_failure_is_retained() {
        let (mut file, profile, cfg) = fixture();
        let reads = Arc::new(AtomicUsize::new(0));
        let calls = reads.clone();
        let resources = NativeResources::blackcyc(
            ClientId(1),
            Arc::new(move |_, _| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(None)
            }),
        )
        .unwrap();
        assert!(SavedPictures::new(ClientId(2), &file, &profile, &cfg, resources.clone()).is_err());
        file.blocks[6].payload[0..256].fill(b'x');
        assert!(SavedPictures::new(ClientId(1), &file, &profile, &cfg, resources.clone()).is_err());
        assert_eq!(reads.load(Ordering::SeqCst), 0);
        file.blocks[6].payload[0..256].fill(0);
        name(&mut file, 0, b"bgcue");
        word(&mut file, 0, 0, 1);
        let (_dir, mut runtime) = runtime();
        let scene = serde_json::to_value(&runtime.host.engine).unwrap();
        let mut prepared =
            SavedPictures::new(ClientId(1), &file, &profile, &cfg, resources).unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        let error = loop {
            match prepared.poll(&mut runtime.host) {
                Err(error) => break error,
                Ok(false) => {
                    assert!(Instant::now() < end);
                    std::thread::yield_now();
                }
                Ok(true) => panic!("missing picture became a success"),
            }
        };
        assert!(error.to_string().contains("absent"));
        let count = reads.load(Ordering::SeqCst);
        assert!(prepared
            .poll(&mut runtime.host)
            .unwrap_err()
            .to_string()
            .contains("absent"));
        assert_eq!(reads.load(Ordering::SeqCst), count);
        assert!(prepared.into_prepared().is_err());
        assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), scene);
    }
}
