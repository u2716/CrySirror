//! Native installation-key UI policy. This validates a supplied installation
//! key; it neither generates keys nor claims online activation succeeded.
use super::*;
const ROOT: u32 = 0x38f0_0000;
const FIELD: u32 = ROOT + 1;
const OK: u32 = ROOT + 2;
const CANCEL: u32 = ROOT + 3;
const ERROR: u32 = ROOT + 4;

fn number(bytes: &[u8]) -> i32 {
    bytes.iter().fold(0i32, |value, byte| {
        value
            .wrapping_mul(36)
            .wrapping_add(if byte.is_ascii_digit() {
                i32::from(*byte) - 22
            } else {
                i32::from(byte.to_ascii_uppercase()) - 65
            })
    })
}
/// Original44D1F0 error codes are part of the native validation policy.
fn validate_install_key(bytes: &[u8], product: i32) -> std::result::Result<i32, i32> {
    if bytes.len() != 11 {
        return Err(-4);
    }
    const PERMUTATION: [usize; 11] = [0, 1, 5, 3, 10, 8, 7, 2, 6, 9, 4];
    let decoded = PERMUTATION.map(|index| bytes[index].to_ascii_uppercase());
    let product_field = number(&decoded[..2]);
    let serial = number(&decoded[2..7]);
    let checksum = number(&decoded[7..]);
    let crc = i32::from(crysirror_resource::checksum::crc16_xmodem(&decoded[..7]));
    if crc + 25 * (crc - crc / 25) != checksum {
        return Err(-1);
    }
    let short = serial % 10_000_000;
    if 10_000_001i32
        .wrapping_mul(short)
        .wrapping_sub(60_000_000i32.wrapping_mul(short / 6))
        != serial
    {
        return Err(-2);
    }
    if product_field != product {
        return Err(-3);
    }
    Ok(short)
}
#[derive(Default)]
pub(super) struct LicenseDialog {
    worker: Option<Receiver<std::result::Result<Option<String>, String>>>,
    loaded: bool,
    opened: bool,
    accepted: Option<String>,
}
impl LicenseDialog {
    pub(super) fn ui_waiting(&self) -> bool {
        self.opened && self.worker.is_none() && self.accepted.is_none()
    }
}
impl Bridge<'_> {
    pub(super) fn native_license(&mut self) -> Result<Option<SystemReply>> {
        let mut state = self.services.system.license.take().unwrap_or_default();
        let result = self.tick_native_license(&mut state);
        if !matches!(result, Ok(Some(_))) {
            self.services.system.license = Some(state);
        }
        result
    }
    fn tick_native_license(&mut self, state: &mut LicenseDialog) -> Result<Option<SystemReply>> {
        if let Some(worker) = &state.worker {
            match worker.try_recv() {
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => {
                    bail!("Circus installation-key worker disconnected")
                }
                Ok(result) => {
                    state.worker = None;
                    let key = result.map_err(anyhow::Error::msg)?;
                    if state.accepted.is_some() {
                        self.submit(vec![], vec![json!({"op":"remove","id":ROOT})])?;
                        return Ok(Some(SystemReply::Done));
                    }
                    state.loaded = true;
                    if key
                        .as_ref()
                        .is_some_and(|key| validate_install_key(key.as_bytes(), 171).is_ok())
                    {
                        return Ok(Some(SystemReply::Done));
                    }
                }
            }
        }
        if !state.loaded || state.accepted.is_some() {
            let storage = self
                .services
                .storage
                .as_ref()
                .context("Circus installation-key storage capability")?
                .clone();
            let accepted = state.accepted.clone();
            let (sender, receiver) = mpsc::channel();
            std::thread::Builder::new()
                .name("circus-installation-key".into())
                .spawn(move || {
                    let result = (|| -> Result<Option<String>> {
                        let path = VfsPath::new(Mount::Save, "circus-installation-key.json")?;
                        if let Some(key) = accepted {
                            storage.write_atomic(
                                &path,
                                &serde_json::to_vec(&json!({"schema":1,"InstKey":key}))?,
                            )?;
                            return Ok(None);
                        }
                        if storage.metadata(Mount::Save, path.relative())?.is_none() {
                            return Ok(None);
                        }
                        let reader = storage.open_read(&path)?;
                        let bytes = crysirror_resource::read_all(reader.as_ref(), 4096)?;
                        let value: Value = serde_json::from_slice(&bytes)?;
                        ensure!(
                            value["schema"] == 1,
                            "Circus installation-key preference schema"
                        );
                        Ok(value["InstKey"].as_str().map(str::to_owned))
                    })();
                    let _ = sender.send(result.map_err(|e| format!("{e:#}")));
                })?;
            state.worker = Some(receiver);
            return Ok(None);
        }
        if !state.opened {
            //410410 enters the original key dialog in windowed mode.
            if self.host.window_fullscreen(self.client)? == Some(true)
                || self.services.system.pending_fullscreen.is_some()
            {
                if self
                    .native_system(SystemRequest::Fullscreen(false))?
                    .is_none()
                {
                    return Ok(None);
                }
            }
            self.submit(vec![],vec![
                json!({"op":"put","widget":{"id":ROOT,"rect":[340,220,600,280],"modal":true,"clip_children":true,"color":[24,24,30,255]}}),
                json!({"op":"put","widget":{"id":FIELD,"parent":ROOT,"rect":[35,50,530,50],"kind":"text_field","text":"","font":self.font,"text_size":30,"color":[255,255,255,255],"text_color":[0,0,0,255],"max_length":11}}),
                json!({"op":"put","widget":{"id":ERROR,"parent":ROOT,"rect":[35,110,530,50],"text":"Enter the installation key supplied with your game.","font":self.font,"text_size":20,"text_color":[255,255,255,255]}}),
                json!({"op":"put","widget":{"id":OK,"parent":ROOT,"rect":[300,200,125,45],"kind":"button","text":"OK","font":self.font,"text_size":24}}),
                json!({"op":"put","widget":{"id":CANCEL,"parent":ROOT,"rect":[440,200,125,45],"kind":"button","text":"Cancel","font":self.font,"text_size":24}}),
                json!({"op":"focus","id":FIELD}),
            ])?;
            state.opened = true;
            return Ok(None);
        }
        let events: Vec<Value> = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::UiSubtreeEvents(ROOT))?,
        )?;
        for event in events {
            let target = if event["widget"]["id"].as_u64() == Some(u64::from(FIELD)) {
                match event["kind"].as_str() {
                    Some("submit") => Some(u64::from(OK)),
                    Some("cancel") => Some(u64::from(CANCEL)),
                    _ => None,
                }
            } else if event["kind"] == "activate" {
                event["widget"]["id"].as_u64()
            } else {
                None
            };
            match target {
                Some(id) if id == u64::from(CANCEL) => {
                    self.submit(vec![], vec![json!({"op":"remove","id":ROOT})])?;
                    return Ok(Some(SystemReply::Cancelled));
                }
                Some(id) if id == u64::from(OK) => {
                    let widget: Value = serde_json::from_slice(
                        &self
                            .host
                            .query_engine(self.client, EngineQuery::UiWidget(FIELD))?,
                    )?;
                    let key = widget["text"]
                        .as_str()
                        .context("Circus installation-key control missing")?;
                    let validation = if key.is_ascii() {
                        validate_install_key(key.as_bytes(), 171)
                    } else {
                        Err(-4)
                    };
                    match validation {
                        Ok(_)=>{state.accepted=Some(key.into());return Ok(None)},
                        Err(code)=>self.submit(vec![],vec![json!({"op":"set_text","id":ERROR,"text":format!("Invalid installation key ({code}).")})])?,
                    }
                }
                _ => {}
            }
        }
        Ok(None)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_alphabet_and_rejection_codes_do_not_accept_arbitrary_keys() {
        assert_eq!(number(b"AZ09"), 33371);
        assert_eq!(number(b"az09"), 33371);
        assert_eq!(validate_install_key(b"", 171), Err(-4));
        assert_eq!(validate_install_key(b"AAAAAAAAAA", 171), Err(-4));
        assert_eq!(validate_install_key(b"AAAAAAAAAAA", 171), Err(-1));
        assert_eq!(validate_install_key(b"aaaaaaaaaaa", 171), Err(-1));
    }
}
