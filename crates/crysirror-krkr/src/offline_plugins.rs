// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Store-launcher and draw-device plug-ins whose external service is absent.
//!
//! Member names are the shipped DLLs' UTF-16 registration strings
//! (inspected 2026-11-15; nothing executed).
//!
//! * dmmcloud.dll `DMMCloud`: the DMM GAME PLAYER IPC methods answer false
//!   because no launcher is attached; `dcgp_game` is the product-id string.
//! * krkrsteam.dll `Steam`: the game was not started by Steam, which the
//!   original reports as `requestInitialize()` false and `initialized` 0.
//!   Achievements/cloud/screenshot/broadcast calls answer false or empty
//!   records with every documented field; `getLanguage()` maps the
//!   configured user locale to Steam's language names. `new Steam()` fails
//!   with "Cannot create instance" like the original static class.
//! * DrawDeviceD2D.dll `DrawDeviceD2D`, SteamDrawDevice.dll
//!   `DualDrawDevice_2` / `DualDrawDevice_Z`: presentation devices. Each is
//!   the engine's retained-layer presentation (as `Window.BasicDrawDevice`),
//!   so `window.drawDevice = new DrawDeviceD2D()` is accepted and pixels are
//!   presented by the shared renderer on every platform.
use crate::natives::Rt;
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{NativeArgCount, ObjectHandle, Variant},
};

fn bound_instance(rt: &mut Rt, this: Option<ObjectHandle>, class: &'static str) -> ObjectHandle {
    let instance = this
        .map(|h| rt.bound_this(h).unwrap_or(h))
        .filter(|h| *h != rt.global_handle())
        .unwrap_or_else(|| rt.alloc_ordinary_object());
    rt.add_object_class_info(instance, class);
    instance
}

pub(crate) fn install_dmmcloud(rt: &mut Rt) {
    if rt.global_member("DMMCloud").object_handle().is_some() {
        return;
    }
    fn members(rt: &mut Rt, h: ObjectHandle) {
        for (name, min) in [("open_store", 1), ("launch_ime", 1), ("close_ime", 0), ("new_window", 1), ("new_tab", 1)] {
            rt.register_object_native_with_arg_count(h, name, NativeArgCount::AtLeast(min), |_: &mut Rt, _, _| Ok(Variant::Integer(0)));
        }
        rt.register_object_native(h, "finalize", |_: &mut Rt, _, _| Ok(Variant::Void));
        rt.set_object_member(h, "dcgp_game", Variant::String("dcgp_game".into()));
    }
    let class = rt.alloc_native_constructor(|rt: &mut Rt, this: Option<ObjectHandle>, _| {
        let instance = bound_instance(rt, this, "DMMCloud");
        members(rt, instance);
        Ok(Variant::Object(instance))
    });
    rt.add_object_class_info(class, "DMMCloud");
    members(rt, class);
    rt.set_global_member("DMMCloud", Variant::Object(class));
}

pub(crate) fn install_steam(rt: &mut Rt) {
    if rt.global_member("Steam").object_handle().is_some() {
        return;
    }
    let class = rt.alloc_native_constructor(|_: &mut Rt, _, _| Err(TjsError::runtime("Cannot create instance")));
    rt.add_object_class_info(class, "Steam");
    type F = fn(&mut Rt, Option<ObjectHandle>, Vec<Variant>) -> Result<Variant>;
    let no: F = |_, _, _| Ok(Variant::Integer(0));
    let void: F = |_, _, _| Ok(Variant::Void);
    let record = |fields: &'static [(&'static str, bool)]| {
        move |rt: &mut Rt, _: Option<ObjectHandle>, _: Vec<Variant>| {
            let h = rt.alloc_dictionary_object();
            for (name, text) in fields {
                rt.set_object_member(h, *name, if *text { Variant::String(String::new()) } else { Variant::Integer(0) });
            }
            Ok(Variant::Object(h))
        }
    };
    let methods: &[(&str, usize, F)] = &[
        ("requestInitialize", 0, no),
        ("setAchievement", 1, no),
        ("clearAchievement", 1, no),
        ("deleteCloudFile", 1, no),
        ("copyCloudFile", 2, no),
        ("triggerScreenshot", 0, void),
        ("hookScreenshots", 1, no),
        ("writeScreenshot", 2, void),
        ("isBroadcasting", 0, no),
        ("hookBroadcasting", 1, no),
    ];
    for (name, min, f) in methods {
        rt.register_object_native_with_arg_count(class, *name, NativeArgCount::AtLeast(*min), *f);
    }
    rt.register_object_native_with_arg_count(class, "getAchievement", NativeArgCount::AtLeast(1),
        record(&[("ach", true), ("name", true), ("desc", true), ("achieved", false), ("unlockTime", false)]));
    rt.register_object_native_with_arg_count(class, "getCloudQuota", NativeArgCount::AtLeast(0),
        record(&[("total", false), ("available", false)]));
    rt.register_object_native_with_arg_count(class, "getCloudFileInfo", NativeArgCount::AtLeast(1),
        record(&[("filename", true), ("size", false), ("time", false), ("desc", true)]));
    rt.register_object_native(class, "getLanguage", |rt: &mut Rt, _, _| {
        Ok(Variant::String(steam_language(&crate::system_plugins::user_locale(rt).0).into()))
    });
    for name in ["initialized", "achievementsCount", "cloudFileCount"] {
        rt.register_object_native_property(class, name, |_, _| Ok(Variant::Integer(0)), move |_, _, _| {
            Err(TjsError::runtime(format!("Steam.{name} is read-only")))
        });
    }
    // Without Steam there is no cloud setting to change; the getter keeps 0.
    rt.register_object_native_property(class, "cloudEnabled", |_, _| Ok(Variant::Integer(0)), |_, _, _| Ok(()));
    rt.set_global_member("Steam", Variant::Object(class));
}

/// Steam API language name for a `ja-JP` style locale.
fn steam_language(locale: &str) -> &'static str {
    let mut parts = locale.split('-');
    let language = parts.next().unwrap_or("");
    let region = parts.next().unwrap_or("").to_ascii_uppercase();
    match language {
        "ja" => "japanese",
        "ko" => "koreana",
        "zh" if matches!(region.as_str(), "TW" | "HK" | "MO" | "HANT") => "tchinese",
        "zh" => "schinese",
        "de" => "german",
        "fr" => "french",
        "it" => "italian",
        "es" if region == "419" || region == "MX" => "latam",
        "es" => "spanish",
        "pt" if region == "BR" => "brazilian",
        "pt" => "portuguese",
        "ru" => "russian",
        "th" => "thai",
        "pl" => "polish",
        "da" => "danish",
        "nl" => "dutch",
        "fi" => "finnish",
        "no" | "nb" | "nn" => "norwegian",
        "sv" => "swedish",
        "hu" => "hungarian",
        "cs" => "czech",
        "ro" => "romanian",
        "tr" => "turkish",
        "bg" => "bulgarian",
        "el" => "greek",
        "uk" => "ukrainian",
        "vi" => "vietnamese",
        "id" => "indonesian",
        "ar" => "arabic",
        _ => "english",
    }
}

/// `DrawDeviceD2D` / `DualDrawDevice_2` / `DualDrawDevice_Z`.
pub(crate) fn install_draw_device(rt: &mut Rt, class_name: &'static str) {
    if rt.global_member(class_name).object_handle().is_some() {
        return;
    }
    let class = rt.alloc_native_constructor(move |rt: &mut Rt, this: Option<ObjectHandle>, _| {
        let h = bound_instance(rt, this, class_name);
        rt.set_object_member(h, "__crysirror_device", Variant::Integer(1));
        crate::natives::draw_device_members(rt, h);
        Ok(Variant::Object(h))
    });
    rt.add_object_class_info(class, class_name);
    crate::natives::draw_device_members(rt, class);
    rt.register_object_native(class, "finalize", |_: &mut Rt, _, _| Ok(Variant::Void));
    rt.set_global_member(class_name, Variant::Object(class));
}

#[cfg(test)]
mod tests {
    #[test]
    fn steam_language_names() {
        for (locale, name) in [("ja-JP", "japanese"), ("zh-TW", "tchinese"), ("zh-CN", "schinese"), ("pt-BR", "brazilian"), ("en-US", "english"), ("xx", "english")] {
            assert_eq!(super::steam_language(locale), name);
        }
    }
}

/// win32ole.dll `WIN32OLE` / `ActiveX` (krkr2 trunk `win32ole/main.cpp`). There is no COM: every object behaves
/// as on a Windows installation where the ProgID/CLSID cannot be created —
/// construction succeeds and logs, `invoke`/`set`/`get` fail with
/// `Unknown failure : FFFFFFFF`, `missing` declines (ordinary member
/// semantics), `addEvent` logs the failed registration, `getConstant` does
/// nothing. ActiveX geometry is plain state; no control window exists, so
/// `visible`/`isValidWindow` read 0.
pub(crate) fn install_win32ole(rt: &mut Rt) {
    for (class_name, activex) in [("WIN32OLE", false), ("ActiveX", true)] {
        if rt.global_member(class_name).object_handle().is_some() {
            continue;
        }
        let class = rt.alloc_native_constructor(move |rt: &mut Rt, this: Option<ObjectHandle>, args: Vec<Variant>| {
            let Some(name) = args.first() else { return Err(TjsError::bad_param_count()) };
            let name = name.to_tjs_string()?;
            if activex && let Some(window) = args.get(1).and_then(|v| v.object_handle())
                && !rt.object_class_infos(window).iter().any(|c| c == "Window")
            {
                return Err(TjsError::runtime("must set window object"));
            }
            let h = bound_instance(rt, this, class_name);
            ole_members(rt, h, activex);
            rt.set_object_member(h, "__win32ole_instance", Variant::Integer(1));
            if activex {
                let geometry = if args.len() >= 6 {
                    [args[2].to_integer()?, args[3].to_integer()?, args[4].to_integer()?, args[5].to_integer()?]
                } else {
                    [0, 0, -1, -1]
                };
                for (field, value) in ["left", "top", "width", "height"].iter().zip(geometry) {
                    rt.set_object_member(h, &format!("__ole_{field}"), Variant::Integer(value));
                }
            }
            let step = if activex {
                "CreateControl failed"
            } else if name.starts_with('{') {
                "CoCreateInstance failed"
            } else {
                "bad CLSID"
            };
            rt.host_mut().push_log(format!("win32ole: {step} {name} (no OLE automation on this platform)"));
            Ok(Variant::Object(h))
        });
        rt.add_object_class_info(class, class_name);
        ole_members(rt, class, activex);
        rt.set_global_member(class_name, Variant::Object(class));
    }
}

fn ole_members(rt: &mut Rt, h: ObjectHandle, activex: bool) {
    fn instance(rt: &mut Rt, this: Option<ObjectHandle>) -> Result<ObjectHandle> {
        this.map(|o| rt.bound_this(o).unwrap_or(o))
            .filter(|o| matches!(rt.object_member(*o, "__win32ole_instance"), Variant::Integer(1)))
            .ok_or_else(|| TjsError::runtime("native class crash (no WIN32OLE instance)"))
    }
    for name in ["invoke", "set", "get"] {
        rt.register_object_native(h, name, |rt: &mut Rt, this: Option<ObjectHandle>, _| {
            instance(rt, this)?;
            Err(TjsError::runtime("Unknown failure : FFFFFFFF"))
        });
    }
    rt.register_object_native_with_arg_count(h, "missing", NativeArgCount::AtLeast(3), |rt: &mut Rt, this: Option<ObjectHandle>, _| {
        instance(rt, this)?;
        Ok(Variant::Integer(0))
    });
    rt.register_object_native_with_arg_count(h, "addEvent", NativeArgCount::AtLeast(1), |rt: &mut Rt, this: Option<ObjectHandle>, args: Vec<Variant>| {
        if let Some(receiver) = args.get(1)
            && !matches!(receiver, Variant::Void)
            && receiver.object_handle().is_none()
        {
            return Err(TjsError::runtime("addEvent receiver must be an object"));
        }
        instance(rt, this)?;
        let name = args[0].to_tjs_string()?;
        rt.host_mut().push_log(format!("win32ole: イベント[{name}]の登録に失敗しました"));
        Ok(Variant::Void)
    });
    rt.register_object_native(h, "getConstant", |rt: &mut Rt, this: Option<ObjectHandle>, args: Vec<Variant>| {
        if let Some(target) = args.first()
            && !matches!(target, Variant::Void)
            && target.object_handle().is_none()
        {
            return Err(TjsError::runtime("getConstant target must be an object"));
        }
        instance(rt, this)?;
        Ok(Variant::Void)
    });
    rt.register_object_native(h, "finalize", |_: &mut Rt, _, _| Ok(Variant::Void));
    if !activex {
        return;
    }
    rt.register_object_native(h, "setExternalUI", |rt: &mut Rt, this: Option<ObjectHandle>, _| {
        instance(rt, this)?;
        Ok(Variant::Void)
    });
    for (name, fields) in [("setPos", ["left", "top"]), ("setSize", ["width", "height"])] {
        rt.register_object_native_with_arg_count(h, name, NativeArgCount::AtLeast(2), move |rt: &mut Rt, this: Option<ObjectHandle>, args: Vec<Variant>| {
            let h = instance(rt, this)?;
            for (field, value) in fields.iter().zip(&args) {
                let value = value.to_integer()?;
                rt.set_object_member(h, &format!("__ole_{field}"), Variant::Integer(value));
            }
            Ok(Variant::Void)
        });
    }
    for field in ["left", "top", "width", "height"] {
        rt.register_object_native_property(
            h,
            field,
            move |rt, this| {
                let h = instance(rt, this)?;
                Ok(rt.object_member(h, &format!("__ole_{field}")))
            },
            move |rt, this, value| {
                let h = instance(rt, this)?;
                let value = value.to_integer()?;
                rt.set_object_member(h, &format!("__ole_{field}"), Variant::Integer(value));
                Ok(())
            },
        );
    }
    rt.register_object_native_property(h, "visible", |rt, this| { instance(rt, this)?; Ok(Variant::Integer(0)) }, |rt, this, _| { instance(rt, this)?; Ok(()) });
    rt.register_object_native_property(
        h,
        "isValidWindow",
        |rt, this| { instance(rt, this)?; Ok(Variant::Integer(0)) },
        |_, _, _| Err(TjsError::runtime("ActiveX.isValidWindow is read-only")),
    );
}

/// gamepad.dll (`GamepadPort`): XInput controller ports. The engine exposes
/// no controller devices, so a port enumerates none: `count` is 0,
/// `getController` answers null and `update` has nothing to poll, as on a
/// machine with no pad connected. Member names from the shipped DLL's
/// registration strings (game-kr `plugin/gamepad.dll`).
pub(crate) fn install_gamepad(rt: &mut Rt) {
    if rt.global_member("GamepadPort").object_handle().is_some() {
        return;
    }
    fn members(rt: &mut Rt, h: ObjectHandle) {
        for name in ["initialize", "update", "finalize"] {
            rt.register_object_native(h, name, |_: &mut Rt, _, _| Ok(Variant::Void));
        }
        rt.register_object_native(h, "getController", |_: &mut Rt, _, _| Ok(Variant::Null));
        rt.set_object_member(h, "count", Variant::Integer(0));
    }
    let class = rt.alloc_native_constructor(|rt: &mut Rt, this: Option<ObjectHandle>, _| {
        let instance = bound_instance(rt, this, "GamepadPort");
        members(rt, instance);
        Ok(Variant::Object(instance))
    });
    rt.add_object_class_info(class, "GamepadPort");
    members(rt, class);
    rt.set_global_member("GamepadPort", Variant::Object(class));
}
