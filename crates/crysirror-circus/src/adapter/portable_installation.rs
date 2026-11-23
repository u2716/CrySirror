//! Explicit mounted-resource installation policy, limited to reviewed DC2PC
//! product87/DC3WY product160. Their InstKey reads feed registry/dialogue and
//! entitlement checks (DC2 409570/415480/4157E0/422E60; DC3 40DBE0,
//! 40E0E0..40EA30/410490/4233A0), not MES/CRX/XPCM decryption. A portable
//! host substitutes its own installed-resource admission for that Windows
//! gate; it does not synthesize a valid key, activation, or DVD response.
//!
//! This is a bootstrap-layout/archive validation, not a claim that every
//! unrequested optional resource exists. Later VM reads retain precise path
//! errors. All I/O/index work remains off the event/render owner.
use super::*;

#[derive(Default)]
pub(super) struct Validation {
    worker: Option<Receiver<std::result::Result<(), String>>>,
    ready: bool,
}

pub(super) fn tick(bridge: &mut Bridge<'_>, state: &mut Validation) -> Result<Option<SystemReply>> {
    let profile = bridge
        .services
        .native_profile
        .context("Circus portable installation profile")?;
    ensure!(
        matches!(
            profile,
            crate::Profile::Dc2PlusCommunication | crate::Profile::Dc3WithYou
        ),
        "Circus mounted-resource installation profile has not been reviewed"
    );
    if state.ready {
        return Ok(Some(SystemReply::Done));
    }
    if let Some(worker) = &state.worker {
        return match worker.try_recv() {
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                bail!("Circus mounted-resource validation worker disconnected")
            }
            Ok(result) => {
                state.worker = None;
                result.map_err(anyhow::Error::msg)?;
                state.ready = true;
                Ok(Some(SystemReply::Done))
            }
        };
    }
    let storage = bridge
        .services
        .storage
        .as_ref()
        .context("Circus portable installation storage capability")?
        .clone();
    let client = bridge.client;
    let archive = bridge.services.voice_archive.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("circus-mounted-installation".into())
        .spawn(move || {
            let result = validate(storage, client, profile, archive);
            let _ = sender.send(result.map_err(|error| format!("{error:#}")));
        })?;
    state.worker = Some(receiver);
    Ok(None)
}

fn file(storage: &dyn ClientStorage, name: &str, limit: u64) -> Result<Vec<u8>> {
    let path = VfsPath::new(Mount::Asset, name)?;
    (|| -> Result<Vec<u8>> {
        let metadata = storage
            .metadata(Mount::Asset, path.relative())?
            .with_context(|| format!("missing file {}", path.relative()))?;
        ensure!(!metadata.is_directory, "expected file {}", path.relative());
        let reader = storage.open_read(&path)?;
        ensure!(reader.len()? > 0, "empty resource {}", path.relative());
        crysirror_resource::read_all(reader.as_ref(), limit)
    })()
    .with_context(|| format!("Circus mounted installation {}", path.relative()))
}

fn validate(
    storage: Arc<dyn ClientStorage>,
    client: ClientId,
    profile: crate::Profile,
    cached_archive: Option<Arc<dyn Archive>>,
) -> Result<()> {
    let bytes = file(
        storage.as_ref(),
        "Advdata/MES/start.mes",
        crate::MAX_SCRIPT_BYTES as u64,
    )?;
    let program = Program::parse(&bytes)?;
    //D.C.III (0x8a71) runs on the DC3WY interpreter (dc3_original).
    ensure!(
        program.profile == profile || (program.profile == crate::Profile::Dc3 && profile == crate::Profile::Dc3WithYou),
        "Circus mounted start.mes native ABI differs from detected profile"
    );
    for directory in ["Advdata/MES", "Advdata/GRP"] {
        let names = storage
            .list(Mount::Asset, directory)
            .with_context(|| format!("Circus mounted directory {directory}"))?;
        ensure!(
            !names.is_empty() && names.len() <= 65_535,
            "Circus mounted directory {directory} is empty or exceeds its entry budget"
        );
    }
    match profile {
        crate::Profile::Dc2PlusCommunication => {
            let bytes = file(storage.as_ref(), "DAT/names.dat", 1024 * 1024)?;
            crate::dc2_names::Names::parse(&bytes)?;
        }
        crate::Profile::Dc3WithYou => {
            let bytes = file(storage.as_ref(), "Advdata/GRP/NAMES.DAT", 256 * 1024)?;
            crate::names::Names::parse(&bytes)?;
        }
        _ => bail!("unreviewed Circus mounted installation ABI"),
    }
    let voice = VfsPath::new(Mount::Asset, "Advdata/VOICE.DAT")?;
    if storage.metadata(Mount::Asset, voice.relative())?.is_some() {
        let archive: Arc<dyn Archive> = if let Some(archive) = cached_archive {
            archive
        } else if profile == crate::Profile::Dc2PlusCommunication {
            //The shared reader validates every referenced split-volume range;
            //it reads the index, never the complete gigabyte-sized volumes.
            crate::adapter::dc2_voice::open_archive(storage.clone(), client)?
        } else {
            let context = DecodeContext::new(client);
            Arc::from(
                crysirror_resource::circus_pck::CircusPck
                    .open(storage.open_read(&voice)?, &context)?,
            )
        };
        ensure!(
            !archive.entries().is_empty(),
            "Circus mounted VOICE.DAT has no entries"
        );
    }
    Ok(())
}
