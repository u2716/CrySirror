//! snd methods of 6DC194 (and dsnd 6DBD64) beyond playback control.
//!
//! The sound manager keeps per-channel volume-track (+11), pan, loaded name
//! and two stream policy flags (`pm`: voice +508/+509, consumed by the
//! decoder's gap and end handling, 5830A4/582D74). Queries read the shared
//! mixer's voice observation; values without a host counterpart are kept
//! and reported.
use super::*;
use crysirror_core::service::AudioPreparePhase;

impl QlieVm {
    /// Returns false when `op` is not handled here.
    pub(super) fn snd_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let n = |i: usize| -> Result<i32> {
            a.get(i).map(Value::integer).transpose().map(|v| v.unwrap_or(0))
        };
        let channel = self.objects[id].channel;
        let info = |vm: &Self, h: &mut dyn HostServices| -> Result<Option<_>> {
            if vm.objects[id].channel == 0 {
                Ok(None)
            } else {
                Ok(h.audio_info(vm.client, vm.objects[id].channel)?)
            }
        };
        match op {
            // sv,level (5C49D0 -> 58530C): from the voice position, probe every
            // 0.2 s with 256 bytes of decoded 16-bit PCM (128 samples; the
            // 256-sample sum divides by 256) until the mean magnitude passes
            // `level`, then seek 0.2 s before that probe; a scan that reaches
            // the last 0.2 s seeks to 0. Levels below 1 do nothing.
            "sv" => {
                ensure!(a.len() == 1, "QLIE sound silence skip ABI");
                let level = n(0)?;
                if let Some((owner, ticket)) = &self.audio_seek {
                    ensure!(owner == id, "QLIE sound seek continuation owner");
                    let status = h.audio_seek_preparation(self.client, *ticket)?;
                    return match status.phase {
                        AudioPreparePhase::Pending => Ok(false),
                        AudioPreparePhase::Ready => {
                            h.release_audio_seek(self.client, *ticket)?;
                            self.audio_seek = None;
                            Ok(true)
                        }
                        AudioPreparePhase::Failed => {
                            let error = status.error;
                            h.release_audio_seek(self.client, *ticket)?;
                            self.audio_seek = None;
                            bail!("QLIE sound seek failed: {error:?}");
                        }
                    };
                }
                if level < 1 || channel == 0 {
                    return Ok(true);
                }
                let Some(state) = info(self, h)? else {
                    return Ok(true);
                };
                if self.sound_scan.is_none() {
                    let asset = self.objects[id].asset.clone().context("QLIE sound scan asset")?;
                    let path = VfsPath::new(Mount::Asset, &asset)?;
                    let storage = h.storage(self.client)?;
                    let start = state.position_frames as f64 / f64::from(state.source_rate.max(1));
                    let extension = asset.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
                    let task = self.pool.spawn(move |cancel| {
                        let bytes: Arc<[u8]> = storage.read(&path)?.into();
                        let mut source =
                            crysirror_audio::decoder::open_memory(bytes, extension.as_deref())?;
                        let probe = crysirror_audio::onset::OnsetProbe {
                            start_seconds: start,
                            step_seconds: 0.2,
                            samples: 128,
                            divisor: 256,
                            threshold: level,
                        };
                        let found = crysirror_audio::onset::first_audible(
                            source.as_mut(),
                            probe,
                            &|| Ok(cancel.check()?),
                        )?;
                        let rate = f64::from(source.sample_rate().max(1));
                        let duration = source.frame_count().unwrap_or(0) as f64 / rate;
                        let at = found.filter(|t| duration - 0.2 >= *t).unwrap_or(0.);
                        Ok((at - 0.2).max(0.))
                    })?;
                    self.sound_scan = Some((id.to_owned(), task));
                }
                let (owner, task) = self.sound_scan.as_mut().unwrap();
                ensure!(owner == id, "QLIE sound scan continuation owner");
                let Some(seconds) = task.try_take()? else {
                    return Ok(false);
                };
                self.sound_scan = None;
                let frame = (seconds * f64::from(state.source_rate)) as u64;
                let ticket = h.begin_audio_seek(self.client, channel, frame)?;
                self.audio_seek = Some((id.to_owned(), ticket));
                return Ok(false);
            }
            // t/vtn,type (5C4A1C/5C4B84) select the volume track; ?t/?vtn
            // read it back.
            "t" => {
                ensure!(a.len() == 1, "QLIE sound volume track ABI");
                let property = vec![Value::Int(n(0)?)];
                self.objects.get_mut(id).unwrap().properties.insert("vtn".into(), property);
            }
            "?t" | "?vtn" => {
                let value = self.objects[id]
                    .properties
                    .get("vtn")
                    .and_then(|v| v.first())
                    .cloned()
                    .unwrap_or(Value::Int(0));
                m.result(0, value);
            }
            // dsnd \?p (6DBD64 -> 5C2B40): DirectSound buffer status, 0
            // without a playing buffer, 1 playing, 2 playing and looping.
            "\\?p" => {
                let state = info(self, h)?.map_or(0, |i| {
                    if !i.source_playing {
                        0
                    } else if i.looping {
                        2
                    } else {
                        1
                    }
                });
                m.result(0, Value::Int(state));
            }
            // ?l (5C40A0) and dsnd \?l (5C30B4): the loaded file name.
            "?l" | "\\?l" => {
                let name = self.objects[id].asset.clone().unwrap_or_default();
                m.result_text(0, name);
            }
            // ?pan (5C40D8): the voice pan, -256..256 (last pan request).
            "?pan" => {
                let stored = self.objects[id]
                    .properties
                    .get("pan")
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(0);
                // The mixer reports no pan observation; the last request is
                // the voice pan (SetPan has no implicit motion besides its fade).
                m.result(0, Value::Int(stored));
            }
            // ?nv (5BFC8C): the voice's current amplitude x256.
            "?nv" => {
                let gain = match info(self, h)? {
                    Some(voice) => voice.gain,
                    None => self.objects[id].gain,
                };
                m.result(0, Value::Int((gain * 256.).round() as i32));
            }
            // ?mnv,type (5C0308): the volume-type level x256 (cfg vtv).
            "?mnv" => {
                ensure!(a.len() == 1, "QLIE volume type query ABI");
                let key = format!("vtv/{}", n(0)?);
                let level = self
                    .objects
                    .get("cfg")
                    .and_then(|o| o.properties.get(&key))
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(256);
                m.result(0, Value::Int(level));
            }
            "pm" => {
                ensure!(a.len() == 2, "QLIE stream policy ABI");
                let flags = vec![Value::Int(i32::from(n(0)? == 1)), Value::Int(i32::from(n(1)? == 1))];
                self.objects.get_mut(id).unwrap().properties.insert("pm".into(), flags);
            }
            // ?fo answers the stored filter chain text.
            "?fo" => {
                let text = self.objects[id].properties.get("fo").and_then(|v| v.first()).map(Value::text);
                m.result_text(0, text.unwrap_or_default());
            }
            // tt answers like ?tt plus an empty ResultStr (LABEL_59).
            "tt" => {
                let time = info(self, h)?
                    .map(|i| i.source_frames.unwrap_or(0).saturating_mul(1000) / u64::from(i.source_rate.max(1)))
                    .unwrap_or(0);
                m.result(0, Value::Int(i32::try_from(time)?));
                m.result_text(0, "");
            }
            _ => {
                let _ = channel;
                return Ok(false);
            }
        }
        Ok(true)
    }
}
