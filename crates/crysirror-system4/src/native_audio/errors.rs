// GPL-2.0-or-later. Original MusicSystem7730/16C0/1710/18E0/1990
// stores two low bytes; dispatches IDE/debug output and optional error box.
use super::*;
use crysirror_core::service::EngineQuery;
const ID: u32 = 62466;
pub(super) struct Notice {
    call: ExternalCall,
    logged: bool,
    opened: bool,
}
impl NativeAudio {
    pub(super) fn configure_errors(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            return Ok(None);
        };
        let f = &m.program.libraries[library];
        let function = &f.functions[function];
        if f.name != b"MusicSystem" || function.name != b"SetErrorOutput" {
            return Ok(None);
        }
        ensure!(
            owner == self.owner && self.pending.is_none() && self.sample_read.is_none(),
            "native music error policy owner/pending"
        );
        ensure!(
            function.return_kind == 0,
            "native music error policy return ABI"
        );
        let args = m.hll_arguments(call)?;
        let [
            Argument::Scalar {
                kind: 47,
                bits: ide,
            },
            Argument::Scalar {
                kind: 47,
                bits: message,
            },
        ] = args.as_slice()
        else {
            anyhow::bail!("native music error policy bool ABI")
        };
        let flags = [*ide as u8, *message as u8];
        m.complete_hll(call, HllReturn::Void)?;
        self.error_output = flags;
        Ok(Some(Dispatch::Completed))
    }
    pub(super) fn failed_music(
        &mut self,
        m: &mut Machine,
        call: &ExternalCall,
        key: Key,
        message: &str,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Dispatch> {
        let pending = self.error_notice.get_or_insert_with(|| Notice {
            call: call.clone(),
            logged: false,
            opened: false,
        });
        ensure!(
            pending.call == *call,
            "native music error notice changed receipt"
        );
        if !pending.logged {
            let message = format!("{message} - MusicSystem");
            if self.error_output[0] != 0 {
                let receipt = host.append_log(self.owner, &message)?;
                ensure!(
                    receipt.client == self.owner,
                    "native music IDE foreign log receipt"
                );
            } else {
                // Original fallback is debugger-only OutputDebugStringA, independent of
                // the configured IDE interface and optional modal presentation.
                eprintln!("{message}");
            }
            pending.logged = true;
        }
        if self.error_output[1] != 0 {
            if !pending.opened {
                let sact = bridge
                    .sact
                    .as_ref()
                    .context("native music error font unavailable")?;
                let font = host
                    .font_library(self.owner)?
                    .resolve_alias(&sact.font_aliases[0])?
                    .context("native music error font unpublished")?;
                host.submit_engine(self.owner,1,&serde_json::to_vec(&serde_json::json!({"ui":[{"op":"open_dialog","dialog":{"id":ID,"title":"MusicSystem","message":message,"buttons":[{"id":1,"label":"OK"}],"default":1,"cancel":1,"font":font,"text_size":22}}]}))?)?;
                pending.opened = true;
                return Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1)));
            }
            let bytes = host.query_engine(self.owner, EngineQuery::DialogEvents)?;
            ensure!(bytes.len() <= 65536, "native music modal events budget");
            let events: Vec<serde_json::Value> = serde_json::from_slice(&bytes)?;
            ensure!(events.len() <= 32, "native music modal events count");
            if events.is_empty() {
                return Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1)));
            }
            ensure!(
                events.len() == 1
                    && events[0]["widget"]["owner"].as_u64() == Some(u64::from(self.owner.0))
                    && events[0]["widget"]["id"].as_u64() == Some(u64::from(ID))
                    && events[0]["kind"] == "dialog_result"
                    && events[0]["value"].as_f64() == Some(1.),
                "native music foreign error acknowledgement"
            );
        }
        self.close_commands(host)?;
        self.retire(key, host)?;
        m.complete_hll(call, HllReturn::Scalar(0))?;
        self.error_notice = None;
        Ok(Dispatch::Completed)
    }
    pub(super) fn close_error(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if self.error_notice.as_ref().is_some_and(|n| n.opened) {
            host.submit_engine(
                self.owner,
                1,
                &serde_json::to_vec(&serde_json::json!({"ui":[{"op":"close_dialog","id":ID}]}))?,
            )?;
        }
        self.error_notice = None;
        Ok(())
    }
}
#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;
    use crate::native_audio::tests::{pending, setup};
    #[test]
    fn native_error_policy_retains_low_bytes_and_routes_prepare_failures() {
        let (_dir, mut runtime, bridge, mut audio) = setup();
        let (mut m, call) = pending("SetErrorOutput", &[257, 512]);
        let library = &mut Arc::make_mut(&mut m.program).libraries[0];
        library.name = b"MusicSystem".to_vec();
        library.functions[0].return_kind = 0;
        library.functions[0].arguments = vec![(vec![], 47); 2];
        assert_eq!(
            audio
                .dispatch(ClientId(1), &mut m, &call, &bridge, &mut runtime.host)
                .unwrap(),
            Some(Dispatch::Completed)
        );
        assert_eq!(audio.error_output, [1, 0]);
        assert!(m.stack.is_empty());
        let (mut m, call) = pending("Music_Prepare", &[3, 1]);
        assert_eq!(
            audio
                .failed_music(
                    &mut m,
                    &call,
                    Key {
                        kind: Kind::Music,
                        number: 3
                    },
                    "decode failed",
                    &bridge,
                    &mut runtime.host
                )
                .unwrap(),
            Dispatch::Completed
        );
        assert_eq!(m.stack, [0]);
        assert!(audio.error_notice.is_none());
        assert_eq!(
            runtime
                .host
                .log_snapshot(ClientId(1))
                .unwrap()
                .records
                .last()
                .unwrap()
                .message,
            "decode failed - MusicSystem"
        );
    }
}
