// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original sound configuration representation, not a mixer/routing engine.
//! Native938/714D80 observes config-found state. Its boot-time caller is
//! WndProc5316A0 message40C, before KCS. 71EC50 selects SOUNDCONFIG; 7182B0
//! imports SOUNDGROUP/MASTER/PART declarations. We retain the engine XML AST
//! rather than manufacture a false legacy branch or copy an old sound engine.
//! Unmapped field projections, preferences and playback remain visible native
//! boundaries. Generic gain/routing/decoding must use shared audio services.
use anyhow::{ensure, Context, Result};
use crysirror_core::service::ClientStorage;
use crysirror_resource::xml::XmlDocument;
use std::sync::Arc;

pub(crate) struct SoundDefinition {
    pub source: Option<String>,
    pub document: Option<Arc<XmlDocument>>,
    pub policy: Option<crate::native_sound_policy::SoundPolicy>,
}
impl SoundDefinition {
    pub fn found(&self) -> u32 {
        u32::from(self.document.is_some())
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        let count = |name: &str| {
            self.document.as_ref().and_then(|d| {
                d.unique_child_path(name)
                    .ok()
                    .flatten()
                    .and_then(|n| d.children(n).ok().map(|n| n.count()))
            })
        };
        serde_json::json!({"source":self.source,"found":self.found()!=0,"resident_bytes":self.document.as_ref().map_or(0,|d|d.resident_bytes()),"groups":count("SOUNDGROUP"),"parts":count("PART"),"policy":self.policy.as_ref().map(|p|p.diagnostics()),"representation":"shared_xml_declarations","audio_playback_mapped":false})
    }
}
pub(crate) fn load(
    storage: &dyn ClientStorage,
    check: &dyn Fn() -> Result<()>,
) -> Result<SoundDefinition> {
    check()?;
    let startup = crate::native_settings::optional_asset(storage, "config/startup.xml")?
        .context("CatSystem2 startup missing for sound definition")?;
    let startup = XmlDocument::parse_with_fallback(&startup,encoding_rs::SHIFT_JIS)?;
    let name = startup
        .unique_child_path("SOUNDCONFIG")?
        .map(|n| n.text.as_str())
        .unwrap_or("");
    if name.is_empty() {
        // Older games (Grisaia) keep SOUNDGROUP in startup.xml and have no soundconf.xml.
        let policy = startup.unique_child_path("SOUNDGROUP").ok().flatten().is_some().then(|| crate::native_sound_policy::SoundPolicy::parse(&startup)).transpose()?;
        return Ok(SoundDefinition {
            source: None,
            document: None,
            policy,
        });
    }
    ensure!(
        name == name.trim(),
        "CatSystem2 unsupported sound config path whitespace"
    );
    let path = if name.contains(['\\', '/']) {
        let path = name.replace('\\', "/");
        ensure!(
            path.split('/')
                .next()
                .is_some_and(|p| p.to_ascii_lowercase().ends_with(".int")),
            "CatSystem2 unsupported qualified sound config path"
        );
        path
    } else {
        let table = startup
            .unique_child_path("INT")?
            .context("CatSystem2 config resource table missing")?;
        let mut archive = None;
        for row in startup.children(table)? {
            let fields = startup.children(row)?.collect::<Vec<_>>();
            if fields
                .iter()
                .any(|n| n.name == "name" && n.text.eq_ignore_ascii_case("config"))
            {
                archive = fields
                    .iter()
                    .find(|n| n.name == "int")
                    .map(|n| n.text.clone());
                break;
            }
        }
        let archive = archive.context("CatSystem2 config resource binding missing")?;
        ensure!(
            name.to_ascii_lowercase().ends_with(".xml"),
            "CatSystem2 unsupported sound definition extension"
        );
        format!("{archive}/{name}")
    };
    check()?;
    let bytes = crate::native_settings::optional_native_asset(storage, &startup, &path)?;
    check()?;
    let document = bytes
        .map(|bytes| XmlDocument::parse_with_fallback(&bytes,encoding_rs::SHIFT_JIS).map(Arc::new))
        .transpose()?;
    let mut policy = document
        .as_ref()
        .map(|d| crate::native_sound_policy::SoundPolicy::parse(d))
        .transpose()?;
    //7182B0 imports user preferences only from client-owned setup.xml. All
    //storage/decoding work stays on this preparation worker.
    if let Some(policy) = policy.as_mut() {
        use crysirror_core::{
            path::{Mount, VfsPath},
            Error,
        };
        match storage.open_read(&VfsPath::new(Mount::Save, "setup.xml")?) {
            Ok(reader) => {
                let bytes = crysirror_resource::read_all(reader.as_ref(), 1024 * 1024)?;
                policy.overlay(&XmlDocument::parse_with_fallback(&bytes,encoding_rs::SHIFT_JIS)?)?;
            }
            Err(Error::NotFound) => {}
            Err(e) => return Err(e.into()),
        }
    }
    check()?;
    Ok(SoundDefinition {
        source: Some(path),
        document,
        policy,
    })
}
