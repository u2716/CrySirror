// SPDX-License-Identifier: MPL-2.0
//! `UnityEvent` persistent listeners (`m_PersistentCalls.m_Calls`) as
//! serialized by the editor, e.g. `Button.m_OnClick`. Invocation is the
//! consumer's: built-in targets (`Animator.Play`, `GameObject.SetActive`,
//! `Selectable.set_interactable`) map onto engine state, script targets
//! onto the VM's ported methods.
use crate::managed::{Fields, Value};
use crate::PPtr;

/// `PersistentListenerMode`.
#[derive(Clone, Debug, PartialEq)]
pub enum Argument {
    /// `EventDefined`: the event's own arguments.
    Event,
    Void,
    Object(PPtr),
    Int(i64),
    Float(f32),
    String(String),
    Bool(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PersistentCall {
    pub target: PPtr,
    /// Assembly-qualified type name without the assembly
    /// (`UnityEngine.Animator`, `AL.Title.TitleScene`).
    pub type_name: String,
    pub method: String,
    pub argument: Argument,
}

/// Runtime-enabled persistent calls of an event field (`m_CallState` Off
/// calls are left out).
pub fn persistent_calls(event: &Fields) -> Vec<PersistentCall> {
    let Some(calls) = event.fields("m_PersistentCalls").and_then(|p| p.array("m_Calls")) else { return Vec::new() };
    calls
        .iter()
        .filter_map(|c| if let Value::Struct(c) = c { Some(c) } else { None })
        .filter(|c| c.int("m_CallState").unwrap_or(2) != 0)
        .map(|c| {
            let a = c.fields("m_Arguments").cloned().unwrap_or_default();
            let argument = match c.int("m_Mode").unwrap_or(1) {
                0 => Argument::Event,
                2 => Argument::Object(a.pptr("m_ObjectArgument").unwrap_or_default()),
                3 => Argument::Int(a.int("m_IntArgument").unwrap_or(0)),
                4 => Argument::Float(a.float("m_FloatArgument").unwrap_or(0.) as f32),
                5 => Argument::String(a.string("m_StringArgument").unwrap_or("").to_owned()),
                6 => Argument::Bool(a.bool("m_BoolArgument").unwrap_or(false)),
                _ => Argument::Void,
            };
            let ty = c.string("m_TargetAssemblyTypeName").unwrap_or("");
            PersistentCall {
                target: c.pptr("m_Target").unwrap_or_default(),
                type_name: ty.split(',').next().unwrap_or("").trim().to_owned(),
                method: c.string("m_MethodName").unwrap_or("").to_owned(),
                argument,
            }
        })
        .collect()
}
