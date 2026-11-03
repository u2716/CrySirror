// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Public scoped channel control, with no native VM or audio device.
use crysirror_core::{service::{AudioParameters,AudioRequest,HostServices,PcmAudio},state::ClientId};
use crysirror_engine::{App,Value};
use crysirror_runtime::Runtime;
use serde_json::json;
#[test]
fn public_exchange_moves_real_channel_voice_and_rejects_bad_or_foreign_scope_atomically()->anyhow::Result<()>{
 let root=tempfile::tempdir()?;let project=serde_json::from_value(json!({"schema":2,"name":"public channel exchange","content_revision":"1","assets":"game","canvas":[4,4],"clients":[{"name":"game","adapter":"crysirror-script-v1","game":"fixture","program":[]}]}))?;
 let runtime=Runtime::with_asset_root(project,root.path(),&root.path().join("state"),Default::default())?;let mut app=App::new(runtime,"game")?;app.init()?;app.start()?;
 app.context().state()?.runtime.host.submit_audio(ClientId(1),AudioRequest::PlayPcm{channel:3,source:PcmAudio::new(48000,vec![[0.5;2];1024])?,parameters:AudioParameters{looping:true,gain:0.5,..Default::default()}})?;
 app.context().state()?.runtime.host.channel_audio.render(1)?;app.context().state()?.runtime.host.channel_audio.take_output();
 let before=app.context().state()?.runtime.host.audio_info(ClientId(1),3)?.unwrap();
 let audio=app.context().singleton("Audio")?;
 app.context().call(audio,"exchangeChannels",&[Value::Int(3),Value::Int(9)])?;
 assert!(app.context().state()?.runtime.host.audio_info(ClientId(1),3)?.is_none());assert_eq!(app.context().state()?.runtime.host.audio_info(ClientId(1),9)?,Some(before));
 for args in [[Value::Int(-1),Value::Int(9)],[Value::Int(9),Value::Int(i64::from(u32::MAX)+1)]]{
  assert!(app.context().call(audio,"exchangeChannels",&args).is_err());assert_eq!(app.context().state()?.runtime.host.audio_info(ClientId(1),9)?,Some(before));
 }
 app.context().state()?.client=ClientId(999);
 assert!(app.context().call(audio,"exchangeChannels",&[Value::Int(9),Value::Int(3)]).is_err());
 app.context().state()?.client=ClientId(1);
 assert_eq!(app.context().state()?.runtime.host.audio_info(ClientId(1),9)?,Some(before));
 app.context().state()?.runtime.host.channel_audio.render(1)?;
 let pcm=app.context().state()?.runtime.host.channel_audio.take_output();assert_eq!(pcm.len(),48);assert!(pcm.iter().all(|p|(p[0]-0.25).abs()<1e-6&&(p[1]-0.25).abs()<1e-6));
 Ok(())
}
