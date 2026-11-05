//! Actual mixer PCM and worker ownership for external logarithmic balance.
use crysirror_audio::{decoder::Clip,mixer::{Mixer,MixerConfig,VoiceSpec},render_job::MixJob};
use crysirror_core::service::AudioBalanceCommand as B;
fn near(a:f32,b:f32){assert!((a-b).abs()<0.000001,"{a} != {b}");}
#[test]
fn balance_uses_db_attenuation_and_only_external_updates_advance_the_track(){
    let mut mixer=Mixer::new(MixerConfig{sample_rate:8000,max_voices:4,seed:1}).unwrap();
    let voice=mixer.play(1,Clip::new(8000,vec![[0.25,0.5];512]).unwrap().source(),VoiceSpec::default()).unwrap();
    mixer.balance_control(1,voice,B::Set{db100:-2000,cancel_track:false}).unwrap();
    mixer.balance_control(1,voice,B::Begin{track:1,target_db100:2000,steps:4,finish_previous:true}).unwrap();
    let mut pcm=[[0.;2];16];mixer.render(&mut pcm).unwrap();for frame in pcm{near(frame[0],0.25);near(frame[1],0.05);}
    assert_eq!(mixer.playback_info(1,voice).unwrap().balance.completed_updates,0);
    mixer.balance_control(1,voice,B::Advance{track:1,updates:1}).unwrap();mixer.render(&mut pcm).unwrap();
    for frame in pcm{near(frame[0],0.25);near(frame[1],0.5*10f32.powf(-0.5));}
    mixer.balance_control(1,voice,B::Advance{track:1,updates:3}).unwrap();mixer.render(&mut pcm).unwrap();
    for frame in pcm{near(frame[0],0.025);near(frame[1],0.5);}
    let observed=mixer.playback_info(1,voice).unwrap().balance;assert!(!observed.active);
    assert!(mixer.balance_control(2,voice,B::Set{db100:0,cancel_track:true}).is_err());
    assert!(mixer.balance_control(1,voice,B::Advance{track:2,updates:4}).is_err());
    assert_eq!(mixer.playback_info(1,voice).unwrap().balance,observed);
}
#[test]
fn render_worker_retains_external_balance_and_publishes_pcm_with_its_observation(){
    let mut mixer=Mixer::new(MixerConfig{sample_rate:8000,max_voices:2,seed:1}).unwrap();
    let voice=mixer.play(1,Clip::new(8000,vec![[0.25,0.5];512]).unwrap().source(),VoiceSpec::default()).unwrap();
    mixer.balance_control(1,voice,B::Set{db100:2000,cancel_track:true}).unwrap();
    let pool=crysirror_tasks::TaskPool::new("balance-test",1,1).unwrap();
    let mut job=MixJob::begin(&pool,&mut [&mut mixer],16,vec![]).unwrap();
    assert_eq!(mixer.playback_info(1,voice).unwrap().balance.db100,2000);
    assert!(mixer.balance_control(1,voice,B::Set{db100:0,cancel_track:true}).is_err());
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(5);
    let pcm=loop{if let Some(pcm)=job.poll(&mut [&mut mixer]).unwrap(){break pcm}assert!(std::time::Instant::now()<deadline);std::thread::yield_now();};
    for frame in pcm{near(frame[0],0.025);near(frame[1],0.5);}
    assert_eq!(mixer.playback_info(1,voice).unwrap().balance.db100,2000);
}
