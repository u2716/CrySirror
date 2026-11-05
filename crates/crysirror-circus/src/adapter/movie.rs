//! Shared media owns the decoder, clock, frame/resource leases and audio. This
//! adapter only retains native52's blocking call and RMB/Escape-release policy.
use super::*;
use crate::machine::movie_ops::MovieRequest;
use crate::machine::text_ops::{TextReply, TextRequest};
use crysirror_core::service::{MediaPhase, MediaRequest, buttons};
const ROOT: u32 = 0x3f00_0000;
const IMAGE: u32 = ROOT + 1;
const MEDIA: u32 = ROOT + 2;
#[derive(Default)]
pub(super) struct NativeMovie {
    name: Option<String>,
    path: Option<VfsPath>,
    image: bool,
    startup_failed:bool,
}
impl NativeMovie {
    pub(super) fn ui_waiting(&self)->bool {self.active()&&self.image}
    pub(super) fn active(&self) -> bool {
        self.name.is_some()
    }
}
impl Bridge<'_> {
    pub(super) fn native_movie(&mut self, request: &MovieRequest) -> Result<Option<TextReply>> {
        match request {
            MovieRequest::Prepare { protected } => {
                if !self
                    .native_effect_control(&crate::machine::effect_ops::EffectRequest::Finish)?
                {
                    return Ok(None);
                }
                if !self
                    .native_effect_control(&crate::machine::effect_ops::EffectRequest::ResetColor)?
                {
                    return Ok(None);
                }
                if !self.falling_effect(0, 0)? {
                    return Ok(None);
                }
                if self
                    .native_text(&TextRequest::FinishDrawableQueues)?
                    .is_none()
                {
                    return Ok(None);
                }
                if !protected {
                    let rate = self.services.text.extra_sprite_rate;
                    self.native_text(&TextRequest::Clear)?;
                    self.services.text.extra_sprite_rate = rate;
                    self.clear_packed_presentation()?;
                    let remove = self
                        .services
                        .displayed
                        .iter()
                        .copied()
                        .filter(|slot| !(37..=44).contains(slot))
                        .collect::<Vec<_>>();
                    self.submit(
                        remove
                            .iter()
                            .map(|slot| json!({"op":"remove","id":0x3700_0000+u32::from(*slot)}))
                            .collect(),
                        vec![],
                    )?;
                    for slot in remove {
                        self.services.displayed.remove(&slot);
                    }
                    let slots = self
                        .services
                        .images
                        .keys()
                        .copied()
                        .filter(|slot| *slot < 51 && !(37..=44).contains(slot))
                        .collect::<Vec<_>>();
                    for slot in slots {
                        if let Some(image) = self.services.images.remove(&slot) {
                            self.host
                                .release_image_resource(self.client, image.path.relative())?;
                        }
                        self.services.image_names.remove(&slot);
                        self.services.image_extents.remove(&slot);
                    }
                }
                Ok(Some(TextReply::Done))
            }
            MovieRequest::Start { name, now: _, gain } => {
                if let Some(active) = &self.services.movie.name {
                    ensure!(active == name, "Circus movie owner changed during playback");
                    return Ok(Some(TextReply::Done));
                }
                let asset = movie_asset(name)?;
                self.root()?;
                let path=match self.host.media_request(self.client,MediaRequest::Play{
                    id:MEDIA,asset,looping:false,audio:true,gain:*gain,
                }) {
                    Ok(path)=>path,
                    Err(crysirror_core::Error::NotFound|crysirror_core::Error::Io(_)|crysirror_core::Error::Unsupported(_))=>{
                        //4535E0/4532C0 release the incomplete graph and the
                        //caller resumes; no modal remains after startup failure.
                        self.services.movie=NativeMovie{name:Some(name.clone()),path:None,image:false,startup_failed:true};
                        return Ok(Some(TextReply::Done));
                    }
                    Err(error)=>return Err(error.into()),
                };
                self.submit(vec![json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,1280,720],"order":[2_000_000_000,0],"paint":{"kind":"solid","color":[0,0,0,255]}}})],vec![json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,1280,720],"z":2_000_000_000,"modal":true,"interaction_only":true,"key_events":[{"buttons":buttons::MOUSE_RIGHT|buttons::ESCAPE,"edge":"release","event":"movie_skip"}]}})])?;
                self.services.movie = NativeMovie {
                    name: Some(name.clone()),
                    path: Some(path),
                    image: false,
                    startup_failed:false,
                };
                Ok(Some(TextReply::Done))
            }
            MovieRequest::Poll { now: _ } => {
                ensure!(
                    self.services.movie.active(),
                    "Circus native movie clock without playback owner"
                );
                if self.services.movie.startup_failed {
                    self.services.movie=NativeMovie::default();
                    return Ok(Some(TextReply::Input{advance:true}));
                }
                let status = self.host.media_info(self.client, MEDIA)?;
                //4535E0 calls4532C0 and returns-1 on startup failure;
                //453610 then restores its host state and resumes the script.
                //The shared media owner performs all decoder/resource cleanup.

                let events: Vec<crysirror_scene::ui::UiEvent> = serde_json::from_slice(
                    &self
                        .host
                        .query_engine(self.client, EngineQuery::UiSubtreeEvents(ROOT))?,
                )?;
                if events.iter().any(|event| event.kind == "movie_skip")
                    || matches!(status.phase, MediaPhase::Failed | MediaPhase::Ended | MediaPhase::Stopped)
                {
                    self.host
                        .media_request(self.client, MediaRequest::Stop { id: MEDIA })?;
                    self.host
                        .media_request(self.client, MediaRequest::Remove { id: MEDIA })?;
                    self.submit(
                        vec![json!({"op":"remove","id":ROOT})],
                        vec![json!({"op":"remove","id":ROOT})],
                    )?;
                    self.services.movie = NativeMovie::default();
                    return Ok(Some(TextReply::Input { advance: true }));
                }
                if status.published_frames > 0 && !self.services.movie.image {
                    ensure!(
                        self.host.media_frame(self.client, MEDIA)?.is_some(),
                        "Circus MPG frame receipt lacks immutable picture"
                    );
                    let path = self
                        .services
                        .movie
                        .path
                        .as_ref()
                        .context("Circus MPG frame surface")?;
                    self.submit(vec![json!({"op":"put","layer":{"id":IMAGE,"parent":ROOT,"rect":[0,0,1280,720],"paint":{"kind":"image","asset":path.relative()}}})],vec![])?;
                    self.services.movie.image = true;
                }
                Ok(Some(TextReply::Input { advance: false }))
            }
        }
    }
}
fn movie_asset(name: &str) -> Result<String> {
    ensure!(
        !name.is_empty() && !name.contains('\0'),
        "Circus MPG resource name"
    );
    let mut name = name.replace('\\', "/");
    if !name.to_ascii_lowercase().ends_with(".mpg") {
        name.push_str(".mpg");
    }
    Ok(
        VfsPath::new(Mount::Asset, &format!("Advdata/MOVIE/{name}"))?
            .relative()
            .into(),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn movie_resource_uses_native_directory_suffix_and_capability_root() {
        assert_eq!(movie_asset("opening").unwrap(), "Advdata/MOVIE/opening.mpg");
        assert_eq!(
            movie_asset("sub\\end.MPG").unwrap(),
            "Advdata/MOVIE/sub/end.MPG"
        );
        assert!(movie_asset("../../escape").is_err());
    }
}
