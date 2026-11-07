// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. Pinned InputString.c plus authorized original InputString
// forwarding exports and System40 input methods40ECF0..40EFC0/4152A0/4153B0.
//! Incremental committed-text policy; the engine owns the queue/IME presenter.
//! No private editor, preedit glyph renderer or synthetic Backspace exists.
use crate::{
    adapter::Dispatch,
    vm::{
        external::{Argument, HllReturn},
        ExternalCall, ExternalTarget, Machine,
    },
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{
        HostServices, TextInputCompositionFont, TextInputSessionHandle, TextInputSessionInfo,
        TextInputSessionSpec,
    },
    state::ClientId,
};
struct Pending {
    call: ExternalCall,
    revision: u64,
}
pub(crate) struct NativeInputString {
    owner: ClientId,
    handle: Option<TextInputSessionHandle>,
    spec: TextInputSessionSpec,
    pending: Option<Pending>,
}
impl NativeInputString {
    pub(crate) fn new(owner: ClientId) -> Self {
        Self {
            owner,
            handle: None,
            spec: TextInputSessionSpec::default(),
            pending: None,
        }
    }
    pub(crate) fn has_state(&self) -> bool {
        self.handle.is_some() || self.pending.is_some()
    }
    pub(crate) fn close(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(handle) = self.handle {
            host.release_text_input_session(self.owner, handle)?;
            self.handle = None;
        }
        self.pending = None;
        Ok(())
    }
    fn info(&self, host: &dyn HostServices) -> Result<TextInputSessionInfo> {
        let handle = self.handle.context("System4 text input session missing")?;
        let info = host.text_input_session_info(self.owner, handle)?;
        ensure!(
            handle.client == self.owner && info.handle == handle,
            "System4 foreign/stale text input receipt"
        );
        Ok(info)
    }
    fn ensure_session(&mut self, host: &mut dyn HostServices) -> Result<TextInputSessionHandle> {
        if self.handle.is_none() {
            let handle = host.open_text_input_session(self.owner, self.spec.clone())?;
            ensure!(
                handle.client == self.owner,
                "System4 foreign text input session"
            );
            self.handle = Some(handle);
        }
        Ok(self.handle.unwrap())
    }
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"session":self.handle.is_some(),"capture":self.spec.capture,"ime_allowed":self.spec.ime_allowed,"awaiting_presenter":self.pending.is_some(),"cursor_area":self.spec.cursor_area,"composition_font":self.spec.composition_font.as_ref().map(|f|serde_json::json!({"family":f.family,"height":f.height,"weight":f.weight,"charset":f.charset}))})
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending InputString target"
            );
            return Ok(None);
        };
        let program = m.program.clone();
        let library = &program.libraries[library];
        let f = &library.functions[function];
        if library.name != b"InputString" {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending InputString library"
            );
            return Ok(None);
        }
        ensure!(owner == self.owner, "System4 foreign InputString owner");
        let args = m.hll_arguments(call)?;
        let name = f.name.as_slice();
        ensure!(
            matches!(
                name,
                b"SetFont"
                    | b"SetPos"
                    | b"Begin"
                    | b"End"
                    | b"OpenIME"
                    | b"CloseIME"
                    | b"GetResultString"
                    | b"ClearResultString"
            ),
            "unsupported original InputString export"
        );
        ensure!(
            f.return_kind == if name == b"GetResultString" { 12 } else { 0 },
            "System4 InputString return ABI"
        );
        ensure!(
            match (name, args.as_slice()) {
                (
                    b"SetFont",
                    [Argument::Scalar { kind: 10, .. }, Argument::String { .. }, Argument::Scalar { kind: 10, .. }],
                ) => true,
                (
                    b"SetPos",
                    [Argument::Scalar { kind: 10, .. }, Argument::Scalar { kind: 10, .. }],
                ) => true,
                (
                    b"Begin" | b"End" | b"OpenIME" | b"CloseIME" | b"GetResultString"
                    | b"ClearResultString",
                    [],
                ) => true,
                _ => false,
            },
            "System4 InputString argument ABI"
        );
        if let Some(pending) = &self.pending {
            ensure!(
                pending.call == *call,
                "System4 stale InputString presentation"
            );
            let info = self.info(host)?;
            ensure!(
                info.revision == pending.revision && info.spec == self.spec,
                "System4 changed InputString presented request"
            );
            if info.presented_revision < pending.revision {
                return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
            }
            // SetFont is advisory: the original ignores ImmSetCompositionFontA
            // failure and stores its request. Applied/PlatformManaged/Rejected
            // are real host observations, not invented native bool returns.
            self.pending = None;
            m.complete_hll(call, HllReturn::Void)?;
            return Ok(Some(Dispatch::Completed));
        }
        let handle = self.ensure_session(host)?;
        if name == b"GetResultString" {
            let info = self.info(host)?;
            let (bytes, _, bad) = encoding_rs::SHIFT_JIS.encode(info.committed.as_str());
            ensure!(
                !bad,
                "System4 committed text is outside native CP932 profile"
            );
            let bytes = bytes.into_owned();
            ensure!(
                m.stack.len() < 65536,
                "System4 input string return stack budget"
            );
            // No fallible VM allocation remains after draining the owner queue.
            // No presenter event can interleave this exclusive HostServices step.
            m.heap.check_growth(1, bytes.len().div_ceil(4))?;
            let taken = host.take_text_input_committed(owner, handle)?;
            ensure!(
                taken.as_str() == info.committed.as_str(),
                "System4 text input changed during owner take"
            );
            m.complete_hll(call, HllReturn::String(bytes))?;
            return Ok(Some(Dispatch::Completed));
        }
        if name == b"ClearResultString" {
            host.replace_text_input_committed(owner, handle, String::new())?;
            m.complete_hll(call, HllReturn::Void)?;
            return Ok(Some(Dispatch::Completed));
        }
        let mut spec = self.spec.clone();
        match (name, args.as_slice()) {
            (
                b"SetFont",
                [Argument::Scalar { bits: size, .. }, Argument::String { bytes, .. }, Argument::Scalar { bits: weight, .. }],
            ) => {
                let bytes = bytes.split(|b| *b == 0).next().unwrap();
                ensure!(bytes.len() < 32, "System4 native LOGFONT face bound");
                let family = crysirror_resource::text_stream::decode_shift_jis(bytes, 128)?;
                let height = size
                    .checked_neg()
                    .context("System4 native composition height overflow")?;
                spec.composition_font = Some(TextInputCompositionFont {
                    family,
                    height,
                    weight: *weight,
                    charset: 0x80,
                });
            }
            (b"SetPos", [Argument::Scalar { bits: x, .. }, Argument::Scalar { bits: y, .. }]) => {
                spec.cursor_area = Some([*x, *y, 1, 1])
            }
            (b"Begin", []) => spec.capture = true,
            (b"End", []) => spec.capture = false,
            (b"OpenIME", []) => spec.ime_allowed = true,
            (b"CloseIME", []) => spec.ime_allowed = false,
            _ => unreachable!(),
        }
        spec.validate()?;
        host.configure_text_input_session(owner, handle, spec.clone())?;
        if name == b"Begin" {
            host.replace_text_input_committed(owner, handle, String::new())?;
        }
        // Original End preserves unread committed bytes. Backspace and other
        // control keys stay on the game's normal key route in every phase.
        self.spec = spec;
        let info = self.info(host)?;
        if info.presented_revision < info.revision {
            self.pending = Some(Pending {
                call: call.clone(),
                revision: info.revision,
            });
            Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
        } else {
            m.complete_hll(call, HllReturn::Void)?;
            Ok(Some(Dispatch::Completed))
        }
    }
}
