// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
// Behaviour notes: install is `install`; `System.ModelessOwnerWindow` carries
// the members the shipped DLL registers (UTF-16 registration strings: `show`,
// `registerMessageReceiver`, `stayOnTop`, inspected 2026-10-28) as a virtual
// owner-window record, matching the windowEx virtual-window profile.
//! k2compat.dll: `Window.TouchMouse` and `System.ModelessOwnerWindow`.
//!
//! `Window.TouchMouse` is the constructible `WindowTouchMouse` class object;
//! instances (and the class) carry `enabled` and five `onTouchMouse*`
//! callbacks that forward to `Window.mainWindow`'s same-kind pointer event
//! with integer arguments (onMouseDown/Up/Move, onClick, onDoubleClick).
//! CrySirror's pointer input reaches the Window/Layer mouse callbacks
//! directly, so the engine itself never raises onTouchMouse*.
//!
//! `ModelessOwnerWindow` is the hidden owner of modeless dialogs. There is no
//! OS window here: `show(visible)` and `stayOnTop` record the owner state the
//! scripts read back, and `registerMessageReceiver` records the receiver;
//! no Win32 message exists to deliver to it.
use crate::host::KrkrHost;
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{NativeArgCount, ObjectHandle, Runtime, Variant},
};

const TOUCH_MOUSE_EVENTS: &[(&str, usize, &str)] = &[
    ("onTouchMouseDown", 4, "onMouseDown"),
    ("onTouchMouseUp", 4, "onMouseUp"),
    ("onTouchMouseMove", 3, "onMouseMove"),
    ("onTouchMouseClick", 2, "onClick"),
    ("onTouchMouseDblClick", 2, "onDoubleClick"),
];

pub fn install(runtime: &mut Runtime<KrkrHost>) {
    install_window_touch_mouse(runtime);
    install_modeless_owner_window(runtime);
}

fn install_window_touch_mouse(runtime: &mut Runtime<KrkrHost>) {
    let Some(window) = runtime.global_member("Window").object_handle() else { return };
    if runtime.object_member(window, "TouchMouse").object_handle().is_some() {
        return;
    }
    let touch_mouse = runtime.alloc_native_constructor(
        |runtime: &mut Runtime<KrkrHost>, this_obj: Option<ObjectHandle>, _args: Vec<Variant>| {
            let instance = bound_instance(runtime, this_obj, "WindowTouchMouse");
            install_touch_mouse_members(runtime, instance);
            Ok(Variant::Object(instance))
        },
    );
    runtime.add_object_class_info(touch_mouse, "WindowTouchMouse");
    install_touch_mouse_members(runtime, touch_mouse);
    runtime.set_object_member(window, "TouchMouse", Variant::Object(touch_mouse));
}

fn install_touch_mouse_members(runtime: &mut Runtime<KrkrHost>, handle: ObjectHandle) {
    runtime.set_object_member(handle, "__enabled", Variant::Integer(0));
    runtime.register_object_native_property(
        handle,
        "enabled",
        |runtime: &mut Runtime<KrkrHost>, this_obj: Option<ObjectHandle>| {
            let enabled = this_obj
                .map(|object| runtime.bound_this(object).unwrap_or(object))
                .and_then(|object| match runtime.object_member(object, "__enabled") {
                    Variant::Integer(value) => Some(value),
                    _ => None,
                })
                .unwrap_or(0);
            Ok(Variant::Integer(enabled))
        },
        |runtime: &mut Runtime<KrkrHost>, this_obj: Option<ObjectHandle>, value: Variant| {
            if let Some(object) = this_obj.map(|object| runtime.bound_this(object).unwrap_or(object)) {
                runtime.set_object_member(object, "__enabled", Variant::Integer(i64::from(value.is_truthy())));
            }
            Ok(())
        },
    );
    for (event, arg_count, target) in TOUCH_MOUSE_EVENTS {
        let target = *target;
        runtime.register_object_native_with_arg_count(
            handle,
            *event,
            NativeArgCount::AtLeast(*arg_count),
            move |runtime: &mut Runtime<KrkrHost>, _this_obj: Option<ObjectHandle>, args: Vec<Variant>| {
                let Some(window) = main_window(runtime) else { return Ok(Variant::Void) };
                let handler = runtime.resolve_object_member(window, target)?;
                if matches!(handler, Variant::Void) {
                    return Ok(Variant::Void);
                }
                let forwarded = args
                    .iter()
                    .map(Variant::to_integer)
                    .collect::<Result<Vec<i64>>>()?
                    .into_iter()
                    .map(Variant::Integer)
                    .collect();
                runtime.call_function(handler, forwarded)?;
                Ok(Variant::Void)
            },
        );
    }
}

fn main_window(runtime: &mut Runtime<KrkrHost>) -> Option<ObjectHandle> {
    let class = runtime.global_member("Window").object_handle()?;
    runtime.resolve_object_member(class, "mainWindow").ok()?.object_handle()
}

fn install_modeless_owner_window(runtime: &mut Runtime<KrkrHost>) {
    let Some(system) = runtime.global_member("System").object_handle() else { return };
    if runtime.object_member(system, "ModelessOwnerWindow").object_handle().is_some() {
        return;
    }
    let class = runtime.alloc_native_constructor(
        |runtime: &mut Runtime<KrkrHost>, this_obj: Option<ObjectHandle>, _args: Vec<Variant>| {
            let instance = bound_instance(runtime, this_obj, "ModelessOwnerWindow");
            runtime.set_object_member(instance, "__visible", Variant::Integer(0));
            runtime.set_object_member(instance, "__stay_on_top", Variant::Integer(0));
            owner_members(runtime, instance);
            Ok(Variant::Object(instance))
        },
    );
    runtime.add_object_class_info(class, "ModelessOwnerWindow");
    owner_members(runtime, class);
    runtime.set_object_member(system, "ModelessOwnerWindow", Variant::Object(class));
}

fn owner_members(runtime: &mut Runtime<KrkrHost>, class: ObjectHandle) {
    runtime.register_object_native(class, "show", |runtime: &mut Runtime<KrkrHost>, this_obj: Option<ObjectHandle>, args: Vec<Variant>| {
        let this = this_obj.map(|o| runtime.bound_this(o).unwrap_or(o)).ok_or_else(|| TjsError::runtime("ModelessOwnerWindow.show"))?;
        let visible = args.first().is_none_or(Variant::is_truthy);
        runtime.set_object_member(this, "__visible", Variant::Integer(i64::from(visible)));
        Ok(Variant::Void)
    });
    runtime.register_object_native(class, "registerMessageReceiver", |runtime: &mut Runtime<KrkrHost>, this_obj: Option<ObjectHandle>, args: Vec<Variant>| {
        let this = this_obj.map(|o| runtime.bound_this(o).unwrap_or(o)).ok_or_else(|| TjsError::runtime("ModelessOwnerWindow.registerMessageReceiver"))?;
        let mut items = runtime
            .object_member(this, "__receivers")
            .object_handle()
            .and_then(|list| runtime.array_elements(list).map(<[Variant]>::to_vec))
            .unwrap_or_default();
        let entry = runtime.alloc_array_object(args);
        items.push(Variant::Object(entry));
        let list = runtime.alloc_array_object(items);
        runtime.set_object_member(this, "__receivers", Variant::Object(list));
        Ok(Variant::Void)
    });
    runtime.register_object_native_property(
        class,
        "stayOnTop",
        |runtime: &mut Runtime<KrkrHost>, this_obj: Option<ObjectHandle>| {
            let this = this_obj.map(|o| runtime.bound_this(o).unwrap_or(o));
            Ok(this.map_or(Variant::Integer(0), |o| match runtime.object_member(o, "__stay_on_top") {
                Variant::Integer(v) => Variant::Integer(v),
                _ => Variant::Integer(0),
            }))
        },
        |runtime: &mut Runtime<KrkrHost>, this_obj: Option<ObjectHandle>, value: Variant| {
            if let Some(this) = this_obj.map(|o| runtime.bound_this(o).unwrap_or(o)) {
                runtime.set_object_member(this, "__stay_on_top", Variant::Integer(i64::from(value.is_truthy())));
            }
            Ok(())
        },
    );
}

fn bound_instance(runtime: &mut Runtime<KrkrHost>, this_obj: Option<ObjectHandle>, class_name: &'static str) -> ObjectHandle {
    let instance = this_obj
        .map(|handle| runtime.bound_this(handle).unwrap_or(handle))
        .filter(|handle| *handle != runtime.global_handle())
        .unwrap_or_else(|| runtime.alloc_ordinary_object());
    runtime.add_object_class_info(instance, class_name);
    instance
}
