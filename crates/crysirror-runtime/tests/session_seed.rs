//! The shared owned seed service works without a native VM, GPU or window.
use crysirror_core::{
    service::{HostServices, SessionSeedQuery, SessionSeedSource, SessionSeedState},
    state::ClientId,
    Error,
};
use crysirror_runtime::{AdapterRegistry, Project, Runtime};
use std::time::{Duration, Instant};
fn runtime() -> anyhow::Result<(tempfile::TempDir, Runtime)> {
    let root = tempfile::tempdir()?;
    let project: Project = serde_json::from_value(
        serde_json::json!({"schema":2,"name":"owned game seed","content_revision":"1","assets":"game","canvas":[4,4],"clients":[{"name":"a","adapter":"crysirror-script-v1","game":"unused"},{"name":"b","adapter":"crysirror-script-v1","game":"unused"}]}),
    )?;
    let rt = Runtime::with_asset_root(
        project,
        root.path(),
        &root.path().join("state"),
        AdapterRegistry::default(),
    )?;
    Ok((root, rt))
}
fn ready(query: &mut dyn SessionSeedQuery) -> anyhow::Result<SessionSeedState> {
    let start = Instant::now();
    loop {
        let state = query.poll().map_err(|e| anyhow::anyhow!("{e}"))?;
        if state != SessionSeedState::Pending {
            return Ok(state);
        }
        anyhow::ensure!(
            start.elapsed() < Duration::from_secs(5),
            "shared seed job timeout"
        );
        std::thread::yield_now();
    }
}
#[test]
fn session_seed_actual_host_admits_explicit_owned_words_without_advancing_runtime() -> anyhow::Result<()> {
    let (_root, mut rt) = runtime()?;
    let before = rt.inspect();
    assert!(matches!(
        rt.host
            .begin_session_seed(ClientId(99), SessionSeedSource::Os { words: 2 }),
        Err(Error::NotFound)
    ));
    assert!(matches!(
        rt.host
            .begin_session_seed(ClientId(1), SessionSeedSource::Os { words: 0 }),
        Err(Error::Invalid(_))
    ));
    let mut a = rt
        .host
        .begin_session_seed(
            ClientId(1),
            SessionSeedSource::Fixed {
                words: vec![0, u32::MAX],
            },
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut b = rt
        .host
        .begin_session_seed(
            ClientId(2),
            SessionSeedSource::Fixed {
                words: vec![11, 13],
            },
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    assert_eq!(a.client(), ClientId(1));
    assert_eq!(b.client(), ClientId(2));
    let SessionSeedState::Ready(first) = ready(a.as_mut())? else {
        anyhow::bail!("actual explicit seed did not complete")
    };
    let SessionSeedState::Ready(second) = ready(b.as_mut())? else {
        anyhow::bail!("other owner seed did not complete")
    };
    assert_eq!(
        (first.client, first.words),
        (ClientId(1), vec![0, u32::MAX])
    );
    assert_eq!((second.client, second.words), (ClientId(2), vec![11, 13]));
    let after = rt.inspect();
    assert_eq!(before["frame"], after["frame"]);
    assert_eq!(before["time_ms"], after["time_ms"]);
    Ok(())
}
#[test]
fn session_seed_actual_os_seed_receipt_has_requested_bounded_shape_and_stays_stable() -> anyhow::Result<()> {
    let (_root, mut rt) = runtime()?;
    let mut query = rt
        .host
        .begin_session_seed(ClientId(1), SessionSeedSource::Os { words: 16 })
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let state = ready(query.as_mut())?;
    let SessionSeedState::Ready(material) = &state else {
        anyhow::bail!("actual OS sampling failed: {state:?}")
    };
    assert_eq!(material.client, ClientId(1));
    assert_eq!(material.words.len(), 16);
    //No assertion about unpredictable values, uniqueness or security strength.
    query.cancel();
    assert_eq!(query.poll().map_err(|e| anyhow::anyhow!("{e}"))?, state);
    Ok(())
}
