#![allow(dead_code)]
// Semantic fallback records for reachable extcalls that have not yet been promoted into extsig.rs.
// These records intentionally use per-entry parameter arrays: each extcall has
// parameter names, ParamKind, display order, return kind, side effects, and evidence.
use super::*;

static ORDER_0: &[usize] = &[];
static ORDER_1: &[usize] = &[0];
static ORDER_2: &[usize] = &[0, 1];
static ORDER_3: &[usize] = &[0, 1, 2];
static ORDER_4: &[usize] = &[0, 1, 2, 3];
static ORDER_5: &[usize] = &[0, 1, 2, 3, 4];
static ORDER_6: &[usize] = &[0, 1, 2, 3, 4, 5];
static ORDER_7: &[usize] = &[0, 1, 2, 3, 4, 5, 6];
static ORDER_8: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 7];

// Reachable semantic fallback for Game extcall 0002_0000 text_init.
static AUTO_POP_0002_0000: &[ParamSpec] = &[
    ParamSpec {
        name: "x",
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for text_init",
    },
    ParamSpec {
        name: "y",
        kind: ParamKind::CoordinateY,
        meaning: "runtime pop-order parameter for text_init",
    },
    ParamSpec {
        name: "width",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for text_init",
    },
    ParamSpec {
        name: "height",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for text_init",
    },
    ParamSpec {
        name: "font_size",
        kind: ParamKind::CoordinateZ,
        meaning: "runtime pop-order parameter for text_init",
    },
    ParamSpec {
        name: "line_spacing",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for text_init",
    },
    ParamSpec {
        name: "mode",
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for text_init",
    },
    ParamSpec {
        name: "flags",
        kind: ParamKind::Flag,
        meaning: "runtime pop-order parameter for text_init",
    },
];
static AUTO_PARAMS_0002_0000: &[Param] = &[
    Param {
        name: "x",
        pop_idx: 0,
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for text_init",
    },
    Param {
        name: "y",
        pop_idx: 1,
        kind: ParamKind::CoordinateY,
        meaning: "runtime pop-order parameter for text_init",
    },
    Param {
        name: "width",
        pop_idx: 2,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for text_init",
    },
    Param {
        name: "height",
        pop_idx: 3,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for text_init",
    },
    Param {
        name: "font_size",
        pop_idx: 4,
        kind: ParamKind::CoordinateZ,
        meaning: "runtime pop-order parameter for text_init",
    },
    Param {
        name: "line_spacing",
        pop_idx: 5,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for text_init",
    },
    Param {
        name: "mode",
        pop_idx: 6,
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for text_init",
    },
    Param {
        name: "flags",
        pop_idx: 7,
        kind: ParamKind::Flag,
        meaning: "runtime pop-order parameter for text_init",
    },
];
// Reachable semantic fallback for Game extcall 0002_0001 text_set_icon.
static AUTO_POP_0002_0001: &[ParamSpec] = &[
    ParamSpec {
        name: "icon_slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for text_set_icon",
    },
    ParamSpec {
        name: "resource_name",
        kind: ParamKind::ResourceStringFromFileDat,
        meaning: "runtime pop-order parameter for text_set_icon",
    },
    ParamSpec {
        name: "x",
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for text_set_icon",
    },
    ParamSpec {
        name: "y",
        kind: ParamKind::CoordinateY,
        meaning: "runtime pop-order parameter for text_set_icon",
    },
];
static AUTO_PARAMS_0002_0001: &[Param] = &[
    Param {
        name: "icon_slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for text_set_icon",
    },
    Param {
        name: "resource_name",
        pop_idx: 1,
        kind: ParamKind::ResourceStringFromFileDat,
        meaning: "runtime pop-order parameter for text_set_icon",
    },
    Param {
        name: "x",
        pop_idx: 2,
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for text_set_icon",
    },
    Param {
        name: "y",
        pop_idx: 3,
        kind: ParamKind::CoordinateY,
        meaning: "runtime pop-order parameter for text_set_icon",
    },
];
// Reachable semantic fallback for Game extcall 0002_0005 text_set_btn.
static AUTO_POP_0002_0005: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_0005: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_0009 ext_0002_0009.
static AUTO_POP_0002_0009: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_0009: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_000A text_get_time.
static AUTO_POP_0002_000A: &[ParamSpec] = &[ParamSpec {
    name: "text_id",
    kind: ParamKind::TextId,
    meaning: "runtime pop-order parameter for text_get_time",
}];
static AUTO_PARAMS_0002_000A: &[Param] = &[Param {
    name: "text_id",
    pop_idx: 0,
    kind: ParamKind::TextId,
    meaning: "runtime pop-order parameter for text_get_time",
}];
// Reachable semantic fallback for Game extcall 0002_000B text_window_set_alpha.
static AUTO_POP_0002_000B: &[ParamSpec] = &[
    ParamSpec {
        name: "window_id",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for text_window_set_alpha",
    },
    ParamSpec {
        name: "alpha",
        kind: ParamKind::Alpha,
        meaning: "runtime pop-order parameter for text_window_set_alpha",
    },
];
static AUTO_PARAMS_0002_000B: &[Param] = &[
    Param {
        name: "window_id",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for text_window_set_alpha",
    },
    Param {
        name: "alpha",
        pop_idx: 1,
        kind: ParamKind::Alpha,
        meaning: "runtime pop-order parameter for text_window_set_alpha",
    },
];
// Reachable semantic fallback for Game extcall 0002_000C text_voice_play.
static AUTO_POP_0002_000C: &[ParamSpec] = &[ParamSpec {
    name: "voice_id",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for text_voice_play",
}];
static AUTO_PARAMS_0002_000C: &[Param] = &[Param {
    name: "voice_id",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for text_voice_play",
}];
// Reachable semantic fallback for Game extcall 0002_000D ext_0002_000D.
static AUTO_POP_0002_000D: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_000D: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_000E text_set_icon_animation_time.
static AUTO_POP_0002_000E: &[ParamSpec] = &[
    ParamSpec {
        name: "duration_ms",
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for text_set_icon_animation_time",
    },
    ParamSpec {
        name: "frame_count",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for text_set_icon_animation_time",
    },
];
static AUTO_PARAMS_0002_000E: &[Param] = &[
    Param {
        name: "duration_ms",
        pop_idx: 0,
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for text_set_icon_animation_time",
    },
    Param {
        name: "frame_count",
        pop_idx: 1,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for text_set_icon_animation_time",
    },
];
// Reachable semantic fallback for Game extcall 0002_0012 text_n.
static AUTO_POP_0002_0012: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_0012: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_0013 text_cat.
static AUTO_POP_0002_0013: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_0013: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_0014 set_history.
static AUTO_POP_0002_0014: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_0014: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_0015 is_text_visible.
static AUTO_POP_0002_0015: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_0015: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_0016 text_set_base.
static AUTO_POP_0002_0016: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_0016: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_0017 enable_voice_cut.
static AUTO_POP_0002_0017: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_0017: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_0018 is_voice_cut.
static AUTO_POP_0002_0018: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_0018: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_0019 texttimecheckset.
static AUTO_POP_0002_0019: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_0019: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_001A ext_0002_001A.
static AUTO_POP_0002_001A: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_001A: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_001B ext_0002_001B.
static AUTO_POP_0002_001B: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_001B: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_001D textredraw.
static AUTO_POP_0002_001D: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_001D: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_001E set_text_mode.
static AUTO_POP_0002_001E: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_001E: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_001F text_init_visualnovelmode.
static AUTO_POP_0002_001F: &[ParamSpec] = &[];
static AUTO_PARAMS_0002_001F: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0002_0020 text_set_icon_mode.
static AUTO_POP_0002_0020: &[ParamSpec] = &[ParamSpec {
    name: "mode",
    kind: ParamKind::Mode,
    meaning: "runtime pop-order parameter for text_set_icon_mode",
}];
static AUTO_PARAMS_0002_0020: &[Param] = &[Param {
    name: "mode",
    pop_idx: 0,
    kind: ParamKind::Mode,
    meaning: "runtime pop-order parameter for text_set_icon_mode",
}];
// Reachable semantic fallback for Game extcall 0003_0007 set_priority.
static AUTO_POP_0003_0007: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for set_priority",
}];
static AUTO_PARAMS_0003_0007: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for set_priority",
}];
// Reachable semantic fallback for Game extcall 0003_0008 ext_0003_0008.
static AUTO_POP_0003_0008: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for ext_0003_0008",
}];
static AUTO_PARAMS_0003_0008: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for ext_0003_0008",
}];
// Reachable semantic fallback for Game extcall 0003_0009 sp_set_center.
static AUTO_POP_0003_0009: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0009: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_000B sp_cls_ex.
static AUTO_POP_0003_000B: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for sp_cls_ex",
}];
static AUTO_PARAMS_0003_000B: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for sp_cls_ex",
}];
// Reachable semantic fallback for Game extcall 0003_000C set_filter.
static AUTO_POP_0003_000C: &[ParamSpec] = &[
    ParamSpec {
        name: "slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for set_filter",
    },
    ParamSpec {
        name: "value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for set_filter",
    },
];
static AUTO_PARAMS_0003_000C: &[Param] = &[
    Param {
        name: "slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for set_filter",
    },
    Param {
        name: "value",
        pop_idx: 1,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for set_filter",
    },
];
// Reachable semantic fallback for Game extcall 0003_000D sp_cls_transition.
static AUTO_POP_0003_000D: &[ParamSpec] = &[
    ParamSpec {
        name: "slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_cls_transition",
    },
    ParamSpec {
        name: "transition_id",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for sp_cls_transition",
    },
];
static AUTO_PARAMS_0003_000D: &[Param] = &[
    Param {
        name: "slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_cls_transition",
    },
    Param {
        name: "transition_id",
        pop_idx: 1,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for sp_cls_transition",
    },
];
// Reachable semantic fallback for Game extcall 0003_000E sp_move_ex.
static AUTO_POP_0003_000E: &[ParamSpec] = &[
    ParamSpec {
        name: "slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_move_ex",
    },
    ParamSpec {
        name: "x",
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for sp_move_ex",
    },
    ParamSpec {
        name: "y",
        kind: ParamKind::CoordinateY,
        meaning: "runtime pop-order parameter for sp_move_ex",
    },
    ParamSpec {
        name: "z",
        kind: ParamKind::CoordinateZ,
        meaning: "runtime pop-order parameter for sp_move_ex",
    },
];
static AUTO_PARAMS_0003_000E: &[Param] = &[
    Param {
        name: "slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_move_ex",
    },
    Param {
        name: "x",
        pop_idx: 1,
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for sp_move_ex",
    },
    Param {
        name: "y",
        pop_idx: 2,
        kind: ParamKind::CoordinateY,
        meaning: "runtime pop-order parameter for sp_move_ex",
    },
    Param {
        name: "z",
        pop_idx: 3,
        kind: ParamKind::CoordinateZ,
        meaning: "runtime pop-order parameter for sp_move_ex",
    },
];
// Reachable semantic fallback for Game extcall 0003_000F sp_set_rect_pos.
static AUTO_POP_0003_000F: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_000F: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0010 ext_0003_0010.
static AUTO_POP_0003_0010: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0010: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0012 sp_set_rotate.
static AUTO_POP_0003_0012: &[ParamSpec] = &[
    ParamSpec {
        name: "slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_set_rotate",
    },
    ParamSpec {
        name: "angle",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for sp_set_rotate",
    },
];
static AUTO_PARAMS_0003_0012: &[Param] = &[
    Param {
        name: "slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_set_rotate",
    },
    Param {
        name: "angle",
        pop_idx: 1,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for sp_set_rotate",
    },
];
// Reachable semantic fallback for Game extcall 0003_0013 face_init.
static AUTO_POP_0003_0013: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0013: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0014 face_set.
static AUTO_POP_0003_0014: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0014: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0015 not_image_sp_get_color.
static AUTO_POP_0003_0015: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0015: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0017 face_cls.
static AUTO_POP_0003_0017: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0017: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0018 sp_set_rect.
static AUTO_POP_0003_0018: &[ParamSpec] = &[
    ParamSpec {
        name: "slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_set_rect",
    },
    ParamSpec {
        name: "left",
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for sp_set_rect",
    },
    ParamSpec {
        name: "top",
        kind: ParamKind::CoordinateY,
        meaning: "runtime pop-order parameter for sp_set_rect",
    },
];
static AUTO_PARAMS_0003_0018: &[Param] = &[
    Param {
        name: "slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_set_rect",
    },
    Param {
        name: "left",
        pop_idx: 1,
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for sp_set_rect",
    },
    Param {
        name: "top",
        pop_idx: 2,
        kind: ParamKind::CoordinateY,
        meaning: "runtime pop-order parameter for sp_set_rect",
    },
];
// Reachable semantic fallback for Game extcall 0003_001A not_image_sp_get_alpha.
static AUTO_POP_0003_001A: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_001A: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_001B not_image_sp_get_rotate.
static AUTO_POP_0003_001B: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_001B: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_001C ext_0003_001C.
static AUTO_POP_0003_001C: &[ParamSpec] = &[
    ParamSpec {
        name: "slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for ext_0003_001C",
    },
    ParamSpec {
        name: "value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0003_001C",
    },
    ParamSpec {
        name: "mode",
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_0003_001C",
    },
    ParamSpec {
        name: "duration_ms",
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0003_001C",
    },
];
static AUTO_PARAMS_0003_001C: &[Param] = &[
    Param {
        name: "slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for ext_0003_001C",
    },
    Param {
        name: "value",
        pop_idx: 1,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0003_001C",
    },
    Param {
        name: "mode",
        pop_idx: 2,
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_0003_001C",
    },
    Param {
        name: "duration_ms",
        pop_idx: 3,
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0003_001C",
    },
];
// Reachable semantic fallback for Game extcall 0003_001D ext_0003_001D.
static AUTO_POP_0003_001D: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for ext_0003_001D",
}];
static AUTO_PARAMS_0003_001D: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for ext_0003_001D",
}];
// Reachable semantic fallback for Game extcall 0003_001E ext_0003_001E.
static AUTO_POP_0003_001E: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for ext_0003_001E",
}];
static AUTO_PARAMS_0003_001E: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for ext_0003_001E",
}];
// Reachable semantic fallback for Game extcall 0003_001F ext_0003_001F.
static AUTO_POP_0003_001F: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_001F: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0020 sp_create.
static AUTO_POP_0003_0020: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for sp_create",
}];
static AUTO_PARAMS_0003_0020: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for sp_create",
}];
// Reachable semantic fallback for Game extcall 0003_0022 ext_0003_0022.
static AUTO_POP_0003_0022: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0022: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0023 ext_0003_0023.
static AUTO_POP_0003_0023: &[ParamSpec] = &[
    ParamSpec {
        name: "slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for ext_0003_0023",
    },
    ParamSpec {
        name: "value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0003_0023",
    },
];
static AUTO_PARAMS_0003_0023: &[Param] = &[
    Param {
        name: "slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for ext_0003_0023",
    },
    Param {
        name: "value",
        pop_idx: 1,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0003_0023",
    },
];
// Reachable semantic fallback for Game extcall 0003_0024 not_image_sp_get_scale.
static AUTO_POP_0003_0024: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for not_image_sp_get_scale",
}];
static AUTO_PARAMS_0003_0024: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for not_image_sp_get_scale",
}];
// Reachable semantic fallback for Game extcall 0003_0025 sp_set_color_0x.
static AUTO_POP_0003_0025: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0025: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0026 sp_bitblt.
static AUTO_POP_0003_0026: &[ParamSpec] = &[
    ParamSpec {
        name: "dst_slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_bitblt",
    },
    ParamSpec {
        name: "src_slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_bitblt",
    },
];
static AUTO_PARAMS_0003_0026: &[Param] = &[
    Param {
        name: "dst_slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_bitblt",
    },
    Param {
        name: "src_slot",
        pop_idx: 1,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_bitblt",
    },
];
// Reachable semantic fallback for Game extcall 0003_0027 sp_set_shake.
static AUTO_POP_0003_0027: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0027: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0028 sp_paint.
static AUTO_POP_0003_0028: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0028: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_002C sp_wait_draw.
static AUTO_POP_0003_002C: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_002C: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_002E sp_show.
static AUTO_POP_0003_002E: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_002E: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_002F sp_hide.
static AUTO_POP_0003_002F: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for sp_hide",
}];
static AUTO_PARAMS_0003_002F: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for sp_hide",
}];
// Reachable semantic fallback for Game extcall 0003_0030 ext_0003_0030.
static AUTO_POP_0003_0030: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0030: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0031 sp_set_child.
static AUTO_POP_0003_0031: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0031: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0032 sp_set_transition.
static AUTO_POP_0003_0032: &[ParamSpec] = &[
    ParamSpec {
        name: "slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_set_transition",
    },
    ParamSpec {
        name: "transition_id",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for sp_set_transition",
    },
    ParamSpec {
        name: "duration_ms",
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for sp_set_transition",
    },
    ParamSpec {
        name: "mode",
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for sp_set_transition",
    },
];
static AUTO_PARAMS_0003_0032: &[Param] = &[
    Param {
        name: "slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for sp_set_transition",
    },
    Param {
        name: "transition_id",
        pop_idx: 1,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for sp_set_transition",
    },
    Param {
        name: "duration_ms",
        pop_idx: 2,
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for sp_set_transition",
    },
    Param {
        name: "mode",
        pop_idx: 3,
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for sp_set_transition",
    },
];
// Reachable semantic fallback for Game extcall 0003_0033 sp_copy_image.
static AUTO_POP_0003_0033: &[ParamSpec] = &[ParamSpec {
    name: "dst_slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for sp_copy_image",
}];
static AUTO_PARAMS_0003_0033: &[Param] = &[Param {
    name: "dst_slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for sp_copy_image",
}];
// Reachable semantic fallback for Game extcall 0003_0034 sp_transition.
static AUTO_POP_0003_0034: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0034: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0035 set_aspect_position_type.
static AUTO_POP_0003_0035: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0035: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0036 get_backbuffer.
static AUTO_POP_0003_0036: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0036: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0037 sp_set_mask.
static AUTO_POP_0003_0037: &[ParamSpec] = &[];
static AUTO_PARAMS_0003_0037: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0003_0038 ext_0003_0038.
static AUTO_POP_0003_0038: &[ParamSpec] = &[
    ParamSpec {
        name: "slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for ext_0003_0038",
    },
    ParamSpec {
        name: "value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0003_0038",
    },
    ParamSpec {
        name: "mode",
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_0003_0038",
    },
];
static AUTO_PARAMS_0003_0038: &[Param] = &[
    Param {
        name: "slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for ext_0003_0038",
    },
    Param {
        name: "value",
        pop_idx: 1,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0003_0038",
    },
    Param {
        name: "mode",
        pop_idx: 2,
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_0003_0038",
    },
];
// Reachable semantic fallback for Game extcall 0004_0004 bgm_get_auto_volume.
static AUTO_POP_0004_0004: &[ParamSpec] = &[];
static AUTO_PARAMS_0004_0004: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0004_0006 bgm_set_auto_volume.
static AUTO_POP_0004_0006: &[ParamSpec] = &[];
static AUTO_PARAMS_0004_0006: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0004_0008 get_bgm_filename.
static AUTO_POP_0004_0008: &[ParamSpec] = &[];
static AUTO_PARAMS_0004_0008: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0004_000B set_master_volume.
static AUTO_POP_0004_000B: &[ParamSpec] = &[];
static AUTO_PARAMS_0004_000B: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0004_000C get_master_volume.
static AUTO_POP_0004_000C: &[ParamSpec] = &[];
static AUTO_PARAMS_0004_000C: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0004_000D mute_master_volume.
static AUTO_POP_0004_000D: &[ParamSpec] = &[];
static AUTO_PARAMS_0004_000D: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0004_000E bgm_mute.
static AUTO_POP_0004_000E: &[ParamSpec] = &[];
static AUTO_PARAMS_0004_000E: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0004_000F mute_bgm_auto_volume.
static AUTO_POP_0004_000F: &[ParamSpec] = &[];
static AUTO_PARAMS_0004_000F: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0005_0006 se_unload.
static AUTO_POP_0005_0006: &[ParamSpec] = &[];
static AUTO_PARAMS_0005_0006: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0005_0007 se_wait.
static AUTO_POP_0005_0007: &[ParamSpec] = &[];
static AUTO_PARAMS_0005_0007: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0005_000E se_mute.
static AUTO_POP_0005_000E: &[ParamSpec] = &[];
static AUTO_PARAMS_0005_000E: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0006_0000 select_init.
static AUTO_POP_0006_0000: &[ParamSpec] = &[ParamSpec {
    name: "selection_id",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for select_init",
}];
static AUTO_PARAMS_0006_0000: &[Param] = &[Param {
    name: "selection_id",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for select_init",
}];
// Reachable semantic fallback for Game extcall 0006_0001 select.
static AUTO_POP_0006_0001: &[ParamSpec] = &[];
static AUTO_PARAMS_0006_0001: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0006_0002 ext_0006_0002.
static AUTO_POP_0006_0002: &[ParamSpec] = &[];
static AUTO_PARAMS_0006_0002: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0006_0003 ext_0006_0003.
static AUTO_POP_0006_0003: &[ParamSpec] = &[];
static AUTO_PARAMS_0006_0003: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0006_0004 select_clear.
static AUTO_POP_0006_0004: &[ParamSpec] = &[];
static AUTO_PARAMS_0006_0004: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0006_0006 select_set_process.
static AUTO_POP_0006_0006: &[ParamSpec] = &[];
static AUTO_PARAMS_0006_0006: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0007_0006 wait_clear.
static AUTO_POP_0007_0006: &[ParamSpec] = &[];
static AUTO_PARAMS_0007_0006: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_0004 btn_hide.
static AUTO_POP_0008_0004: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_0004: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_0005 btn_show.
static AUTO_POP_0008_0005: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_0005: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_0009 ext_0008_0009.
static AUTO_POP_0008_0009: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_0009: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_000A ext_0008_000A.
static AUTO_POP_0008_000A: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_000A: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_000B ext_0008_000B.
static AUTO_POP_0008_000B: &[ParamSpec] = &[
    ParamSpec {
        name: "group",
        kind: ParamKind::ButtonSlot,
        meaning: "runtime pop-order parameter for ext_0008_000B",
    },
    ParamSpec {
        name: "index",
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for ext_0008_000B",
    },
    ParamSpec {
        name: "x",
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for ext_0008_000B",
    },
];
static AUTO_PARAMS_0008_000B: &[Param] = &[
    Param {
        name: "group",
        pop_idx: 0,
        kind: ParamKind::ButtonSlot,
        meaning: "runtime pop-order parameter for ext_0008_000B",
    },
    Param {
        name: "index",
        pop_idx: 1,
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for ext_0008_000B",
    },
    Param {
        name: "x",
        pop_idx: 2,
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for ext_0008_000B",
    },
];
// Reachable semantic fallback for Game extcall 0008_000C ext_0008_000C.
static AUTO_POP_0008_000C: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_000C: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_000D btn_set_toggle.
static AUTO_POP_0008_000D: &[ParamSpec] = &[
    ParamSpec {
        name: "group",
        kind: ParamKind::ButtonSlot,
        meaning: "runtime pop-order parameter for btn_set_toggle",
    },
    ParamSpec {
        name: "index",
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for btn_set_toggle",
    },
];
static AUTO_PARAMS_0008_000D: &[Param] = &[
    Param {
        name: "group",
        pop_idx: 0,
        kind: ParamKind::ButtonSlot,
        meaning: "runtime pop-order parameter for btn_set_toggle",
    },
    Param {
        name: "index",
        pop_idx: 1,
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for btn_set_toggle",
    },
];
// Reachable semantic fallback for Game extcall 0008_000E ext_0008_000E.
static AUTO_POP_0008_000E: &[ParamSpec] = &[
    ParamSpec {
        name: "group",
        kind: ParamKind::ButtonSlot,
        meaning: "runtime pop-order parameter for ext_0008_000E",
    },
    ParamSpec {
        name: "index",
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for ext_0008_000E",
    },
    ParamSpec {
        name: "x",
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for ext_0008_000E",
    },
];
static AUTO_PARAMS_0008_000E: &[Param] = &[
    Param {
        name: "group",
        pop_idx: 0,
        kind: ParamKind::ButtonSlot,
        meaning: "runtime pop-order parameter for ext_0008_000E",
    },
    Param {
        name: "index",
        pop_idx: 1,
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for ext_0008_000E",
    },
    Param {
        name: "x",
        pop_idx: 2,
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for ext_0008_000E",
    },
];
// Reachable semantic fallback for Game extcall 0008_000F btn_enable.
static AUTO_POP_0008_000F: &[ParamSpec] = &[ParamSpec {
    name: "group",
    kind: ParamKind::ButtonSlot,
    meaning: "runtime pop-order parameter for btn_enable",
}];
static AUTO_PARAMS_0008_000F: &[Param] = &[Param {
    name: "group",
    pop_idx: 0,
    kind: ParamKind::ButtonSlot,
    meaning: "runtime pop-order parameter for btn_enable",
}];
// Reachable semantic fallback for Game extcall 0008_0010 btn_set_alpha_0x.
static AUTO_POP_0008_0010: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_0010: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_0012 error_btn_expansion.
static AUTO_POP_0008_0012: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_0012: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_0015 btn_set_anim.
static AUTO_POP_0008_0015: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_0015: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_0016 btn_set_hit.
static AUTO_POP_0008_0016: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_0016: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_0017 btn_get_onmouse.
static AUTO_POP_0008_0017: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_0017: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_0018 btn_anim_clear.
static AUTO_POP_0008_0018: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_0018: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0008_001A btn_onmouse_clear.
static AUTO_POP_0008_001A: &[ParamSpec] = &[];
static AUTO_PARAMS_0008_001A: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0000 skip_set.
static AUTO_POP_0009_0000: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0000: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0001 skip_is.
static AUTO_POP_0009_0001: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0001: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0002 auto_set.
static AUTO_POP_0009_0002: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0002: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0003 auto_is.
static AUTO_POP_0009_0003: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0003: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0004 ext_0009_0004.
static AUTO_POP_0009_0004: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0004: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0005 auto_get_time.
static AUTO_POP_0009_0005: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0005: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0006 ext_0009_0006.
static AUTO_POP_0009_0006: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0006: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0007 ext_0009_0007.
static AUTO_POP_0009_0007: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0007: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0008 ext_0009_0008.
static AUTO_POP_0009_0008: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0008: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0009 ext_0009_0009.
static AUTO_POP_0009_0009: &[ParamSpec] = &[ParamSpec {
    name: "value",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0009_0009",
}];
static AUTO_PARAMS_0009_0009: &[Param] = &[Param {
    name: "value",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0009_0009",
}];
// Reachable semantic fallback for Game extcall 0009_000A ext_0009_000A.
static AUTO_POP_0009_000A: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_000A: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_000C ext_0009_000C.
static AUTO_POP_0009_000C: &[ParamSpec] = &[
    ParamSpec {
        name: "value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0009_000C",
    },
    ParamSpec {
        name: "mode",
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_0009_000C",
    },
];
static AUTO_PARAMS_0009_000C: &[Param] = &[
    Param {
        name: "value",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0009_000C",
    },
    Param {
        name: "mode",
        pop_idx: 1,
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_0009_000C",
    },
];
// Reachable semantic fallback for Game extcall 0009_000E ext_0009_000E.
static AUTO_POP_0009_000E: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_000E: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_000F load_font.
static AUTO_POP_0009_000F: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_000F: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0011 set_language.
static AUTO_POP_0009_0011: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0011: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0012 key_canncel.
static AUTO_POP_0009_0012: &[ParamSpec] = &[
    ParamSpec {
        name: "value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for key_canncel",
    },
    ParamSpec {
        name: "mode",
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for key_canncel",
    },
];
static AUTO_PARAMS_0009_0012: &[Param] = &[
    Param {
        name: "value",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for key_canncel",
    },
    Param {
        name: "mode",
        pop_idx: 1,
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for key_canncel",
    },
];
// Reachable semantic fallback for Game extcall 0009_0013 set_font_color.
static AUTO_POP_0009_0013: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0013: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0014 load_font_ex.
static AUTO_POP_0009_0014: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0014: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0015 ext_0009_0015.
static AUTO_POP_0009_0015: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0015: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0016 ext_0009_0016.
static AUTO_POP_0009_0016: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0016: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0017 ext_0009_0017.
static AUTO_POP_0009_0017: &[ParamSpec] = &[ParamSpec {
    name: "continuation_point",
    kind: ParamKind::Integer,
    meaning: "menu/tab continuation point pushed before ext_0009_0017",
}];
static AUTO_PARAMS_0009_0017: &[Param] = &[Param {
    name: "continuation_point",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "menu/tab continuation point pushed before ext_0009_0017",
}];
// Reachable semantic fallback for Game extcall 0009_0018 ext_0009_0018.
static AUTO_POP_0009_0018: &[ParamSpec] = &[ParamSpec {
    name: "mode",
    kind: ParamKind::Mode,
    meaning: "menu/tab transition mode pushed before ext_0009_0018",
}];
static AUTO_PARAMS_0009_0018: &[Param] = &[Param {
    name: "mode",
    pop_idx: 0,
    kind: ParamKind::Mode,
    meaning: "menu/tab transition mode pushed before ext_0009_0018",
}];
// Reachable semantic fallback for Game extcall 0009_0019 ext_0009_0019.
static AUTO_POP_0009_0019: &[ParamSpec] = &[ParamSpec {
    name: "value",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0009_0019",
}];
static AUTO_PARAMS_0009_0019: &[Param] = &[Param {
    name: "value",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0009_0019",
}];
// Reachable semantic fallback for Game extcall 0009_001A ext_0009_001A.
static AUTO_POP_0009_001A: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_001A: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_001B set_font_size.
static AUTO_POP_0009_001B: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_001B: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_001C get_font_size.
static AUTO_POP_0009_001C: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_001C: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_001D get_font_type.
static AUTO_POP_0009_001D: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_001D: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_001E set_font_effect.
static AUTO_POP_0009_001E: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_001E: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_001F get_font_effect.
static AUTO_POP_0009_001F: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_001F: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0023 input_clear.
static AUTO_POP_0009_0023: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0023: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0024 change_window_size.
static AUTO_POP_0009_0024: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0024: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0030 set_cursor_null.
static AUTO_POP_0009_0030: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0030: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0031 set_hide_cursor_time.
static AUTO_POP_0009_0031: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0031: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0033 scene_skip.
static AUTO_POP_0009_0033: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0033: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0034 ext_0009_0034.
static AUTO_POP_0009_0034: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0034: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0009_0035 ext_0009_0035.
static AUTO_POP_0009_0035: &[ParamSpec] = &[];
static AUTO_PARAMS_0009_0035: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0000 save.
static AUTO_POP_000A_0000: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_0000: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0001 load.
static AUTO_POP_000A_0001: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for load",
}];
static AUTO_PARAMS_000A_0001: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for load",
}];
// Reachable semantic fallback for Game extcall 000A_0004 save_set_thumbnail_size.
static AUTO_POP_000A_0004: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_0004: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0005 thumbnail_set.
static AUTO_POP_000A_0005: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_0005: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0007 save_set_font_size.
static AUTO_POP_000A_0007: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_0007: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0009 is_save.
static AUTO_POP_000A_0009: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for is_save",
}];
static AUTO_PARAMS_000A_0009: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for is_save",
}];
// Reachable semantic fallback for Game extcall 000A_000B savepoint.
static AUTO_POP_000A_000B: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_000B: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_000C save_thumbnail_mosaic_set.
static AUTO_POP_000A_000C: &[ParamSpec] = &[ParamSpec {
    name: "width",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for save_thumbnail_mosaic_set",
}];
static AUTO_PARAMS_000A_000C: &[Param] = &[Param {
    name: "width",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for save_thumbnail_mosaic_set",
}];
// Reachable semantic fallback for Game extcall 000A_000D savetimedraw.
static AUTO_POP_000A_000D: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_000D: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_000E savedaydraw.
static AUTO_POP_000A_000E: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_000E: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_000F save_set_text_rect.
static AUTO_POP_000A_000F: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_000F: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0010 savetextdraw.
static AUTO_POP_000A_0010: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_0010: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0011 get_new_savefile.
static AUTO_POP_000A_0011: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for get_new_savefile",
}];
static AUTO_PARAMS_000A_0011: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for get_new_savefile",
}];
// Reachable semantic fallback for Game extcall 000A_0016 thumbnail_renew.
static AUTO_POP_000A_0016: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_0016: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0018 set_load_after_process.
static AUTO_POP_000A_0018: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_0018: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0019 savesystemdata.
static AUTO_POP_000A_0019: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_0019: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_001A save_set_font_effect.
static AUTO_POP_000A_001A: &[ParamSpec] = &[ParamSpec {
    name: "value",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for save_set_font_effect",
}];
static AUTO_PARAMS_000A_001A: &[Param] = &[Param {
    name: "value",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for save_set_font_effect",
}];
// Reachable semantic fallback for Game extcall 000A_001B save_set_font_color_0x_0x.
static AUTO_POP_000A_001B: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_001B: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_001C delete_file.
static AUTO_POP_000A_001C: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_001C: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_001D save_tmp_dat.
static AUTO_POP_000A_001D: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_001D: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_001E copy_file.
static AUTO_POP_000A_001E: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_001E: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0020 save_lock_not_open_savefileno.
static AUTO_POP_000A_0020: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_0020: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0021 is_save_lock.
static AUTO_POP_000A_0021: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for is_save_lock",
}];
static AUTO_PARAMS_000A_0021: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for is_save_lock",
}];
// Reachable semantic fallback for Game extcall 000A_0022 is_prev_data.
static AUTO_POP_000A_0022: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_0022: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0023 save_point_clear.
static AUTO_POP_000A_0023: &[ParamSpec] = &[];
static AUTO_PARAMS_000A_0023: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000A_0024 save_point_lock.
static AUTO_POP_000A_0024: &[ParamSpec] = &[ParamSpec {
    name: "slot",
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for save_point_lock",
}];
static AUTO_PARAMS_000A_0024: &[Param] = &[Param {
    name: "slot",
    pop_idx: 0,
    kind: ParamKind::SpriteSlot,
    meaning: "runtime pop-order parameter for save_point_lock",
}];
// Reachable semantic fallback for Game extcall 000C_0000 system_btn_set.
static AUTO_POP_000C_0000: &[ParamSpec] = &[
    ParamSpec {
        name: "index",
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for system_btn_set",
    },
    ParamSpec {
        name: "resource_name",
        kind: ParamKind::ResourceStringFromFileDat,
        meaning: "runtime pop-order parameter for system_btn_set",
    },
];
static AUTO_PARAMS_000C_0000: &[Param] = &[
    Param {
        name: "index",
        pop_idx: 0,
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for system_btn_set",
    },
    Param {
        name: "resource_name",
        pop_idx: 1,
        kind: ParamKind::ResourceStringFromFileDat,
        meaning: "runtime pop-order parameter for system_btn_set",
    },
];
// Reachable semantic fallback for Game extcall 000C_0001 system_btn_release.
static AUTO_POP_000C_0001: &[ParamSpec] = &[];
static AUTO_PARAMS_000C_0001: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000C_0002 system_btn_enable.
static AUTO_POP_000C_0002: &[ParamSpec] = &[ParamSpec {
    name: "index",
    kind: ParamKind::CoordinateX,
    meaning: "runtime pop-order parameter for system_btn_enable",
}];
static AUTO_PARAMS_000C_0002: &[Param] = &[Param {
    name: "index",
    pop_idx: 0,
    kind: ParamKind::CoordinateX,
    meaning: "runtime pop-order parameter for system_btn_enable",
}];
// Reachable semantic fallback for Game extcall 000D_0004 set_voice_info.
static AUTO_POP_000D_0004: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_0004: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_0005 voice_enable.
static AUTO_POP_000D_0005: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_0005: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_0006 is_voice_enable.
static AUTO_POP_000D_0006: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_0006: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_0007 ext_000D_0007.
static AUTO_POP_000D_0007: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_0007: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_0008 bgv_play.
static AUTO_POP_000D_0008: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_0008: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_000B get_voice_ex_volume.
static AUTO_POP_000D_000B: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_000B: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_000C set_voice_ex_volume.
static AUTO_POP_000D_000C: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_000C: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_000E voice_autopan_initialize.
static AUTO_POP_000D_000E: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_000E: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_000F voice_autopan_enable.
static AUTO_POP_000D_000F: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_000F: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_0010 set_voice_autopan_size_over.
static AUTO_POP_000D_0010: &[ParamSpec] = &[
    ParamSpec {
        name: "slot",
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for set_voice_autopan_size_over",
    },
    ParamSpec {
        name: "resource_name",
        kind: ParamKind::ResourceStringFromFileDat,
        meaning: "runtime pop-order parameter for set_voice_autopan_size_over",
    },
];
static AUTO_PARAMS_000D_0010: &[Param] = &[
    Param {
        name: "slot",
        pop_idx: 0,
        kind: ParamKind::SpriteSlot,
        meaning: "runtime pop-order parameter for set_voice_autopan_size_over",
    },
    Param {
        name: "resource_name",
        pop_idx: 1,
        kind: ParamKind::ResourceStringFromFileDat,
        meaning: "runtime pop-order parameter for set_voice_autopan_size_over",
    },
];
// Reachable semantic fallback for Game extcall 000D_0011 is_voice_autopan_enable.
static AUTO_POP_000D_0011: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_0011: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_0013 bgv_pause.
static AUTO_POP_000D_0013: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_0013: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_0014 bgv_mute.
static AUTO_POP_000D_0014: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_0014: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_0015 set_bgv_volume.
static AUTO_POP_000D_0015: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_0015: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_0016 get_bgv_volume.
static AUTO_POP_000D_0016: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_0016: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000D_0018 voice_mute.
static AUTO_POP_000D_0018: &[ParamSpec] = &[];
static AUTO_PARAMS_000D_0018: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_0000 history_init_0x_0x.
static AUTO_POP_000E_0000: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_0000: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_0001 historybegin_lpbyte_ptagdata_sztext.
static AUTO_POP_000E_0001: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_0001: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_0002 history_end.
static AUTO_POP_000E_0002: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_0002: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_0003 ext_000E_0003.
static AUTO_POP_000E_0003: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_0003: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_0004 ext_000E_0004.
static AUTO_POP_000E_0004: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_0004: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_0005 history_get_height.
static AUTO_POP_000E_0005: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_0005: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_0006 ext_000E_0006.
static AUTO_POP_000E_0006: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_0006: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_0007 ext_000E_0007.
static AUTO_POP_000E_0007: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_0007: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_0008 ext_000E_0008.
static AUTO_POP_000E_0008: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_0008: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_0009 ext_000E_0009.
static AUTO_POP_000E_0009: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_0009: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_000A history_set_rect.
static AUTO_POP_000E_000A: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_000A: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_000B history_clear.
static AUTO_POP_000E_000B: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_000B: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000E_000C history_set.
static AUTO_POP_000E_000C: &[ParamSpec] = &[];
static AUTO_PARAMS_000E_000C: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000F_0002 set_window_text.
static AUTO_POP_000F_0002: &[ParamSpec] = &[];
static AUTO_PARAMS_000F_0002: &[Param] = &[];
// Reachable semantic fallback for Game extcall 000F_0004 ext_000F_0004.
static AUTO_POP_000F_0004: &[ParamSpec] = &[
    ParamSpec {
        name: "value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_000F_0004",
    },
    ParamSpec {
        name: "mode",
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_000F_0004",
    },
    ParamSpec {
        name: "flags",
        kind: ParamKind::Flag,
        meaning: "runtime pop-order parameter for ext_000F_0004",
    },
];
static AUTO_PARAMS_000F_0004: &[Param] = &[
    Param {
        name: "value",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_000F_0004",
    },
    Param {
        name: "mode",
        pop_idx: 1,
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_000F_0004",
    },
    Param {
        name: "flags",
        pop_idx: 2,
        kind: ParamKind::Flag,
        meaning: "runtime pop-order parameter for ext_000F_0004",
    },
];
// Reachable semantic fallback for Game extcall 000F_0005 ext_000F_0005.
static AUTO_POP_000F_0005: &[ParamSpec] = &[ParamSpec {
    name: "value",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_000F_0005",
}];
static AUTO_PARAMS_000F_0005: &[Param] = &[Param {
    name: "value",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_000F_0005",
}];
// Reachable semantic fallback for Game extcall 0010_0000 effect_stop_skip.
static AUTO_POP_0010_0000: &[ParamSpec] = &[];
static AUTO_PARAMS_0010_0000: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0010_0001 ext_0010_0001.
static AUTO_POP_0010_0001: &[ParamSpec] = &[];
static AUTO_PARAMS_0010_0001: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0010_0002 ext_0010_0002.
static AUTO_POP_0010_0002: &[ParamSpec] = &[
    ParamSpec {
        name: "x",
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for ext_0010_0002",
    },
    ParamSpec {
        name: "y",
        kind: ParamKind::CoordinateY,
        meaning: "runtime pop-order parameter for ext_0010_0002",
    },
];
static AUTO_PARAMS_0010_0002: &[Param] = &[
    Param {
        name: "x",
        pop_idx: 0,
        kind: ParamKind::CoordinateX,
        meaning: "runtime pop-order parameter for ext_0010_0002",
    },
    Param {
        name: "y",
        pop_idx: 1,
        kind: ParamKind::CoordinateY,
        meaning: "runtime pop-order parameter for ext_0010_0002",
    },
];
// Reachable semantic fallback for Game extcall 0011_0000 action_run_count_over.
static AUTO_POP_0011_0000: &[ParamSpec] = &[];
static AUTO_PARAMS_0011_0000: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0011_0001 action_sync_run_count_over.
static AUTO_POP_0011_0001: &[ParamSpec] = &[ParamSpec {
    name: "action_id",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for action_sync_run_count_over",
}];
static AUTO_PARAMS_0011_0001: &[Param] = &[Param {
    name: "action_id",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for action_sync_run_count_over",
}];
// Reachable semantic fallback for Game extcall 0011_0003 action_clear_count_over.
static AUTO_POP_0011_0003: &[ParamSpec] = &[ParamSpec {
    name: "action_id",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for action_clear_count_over",
}];
static AUTO_PARAMS_0011_0003: &[Param] = &[Param {
    name: "action_id",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for action_clear_count_over",
}];
// Reachable semantic fallback for Game extcall 0011_0005 ext_0011_0005.
static AUTO_POP_0011_0005: &[ParamSpec] = &[
    ParamSpec {
        name: "action_id",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
    ParamSpec {
        name: "duration_ms",
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
    ParamSpec {
        name: "from_value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
    ParamSpec {
        name: "to_value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
    ParamSpec {
        name: "mode",
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
    ParamSpec {
        name: "flags",
        kind: ParamKind::Flag,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
];
static AUTO_PARAMS_0011_0005: &[Param] = &[
    Param {
        name: "action_id",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
    Param {
        name: "duration_ms",
        pop_idx: 1,
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
    Param {
        name: "from_value",
        pop_idx: 2,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
    Param {
        name: "to_value",
        pop_idx: 3,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
    Param {
        name: "mode",
        pop_idx: 4,
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
    Param {
        name: "flags",
        pop_idx: 5,
        kind: ParamKind::Flag,
        meaning: "runtime pop-order parameter for ext_0011_0005",
    },
];
// Reachable semantic fallback for Game extcall 0011_0006 ext_0011_0006.
static AUTO_POP_0011_0006: &[ParamSpec] = &[
    ParamSpec {
        name: "action_id",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0006",
    },
    ParamSpec {
        name: "duration_ms",
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_0006",
    },
];
static AUTO_PARAMS_0011_0006: &[Param] = &[
    Param {
        name: "action_id",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0006",
    },
    Param {
        name: "duration_ms",
        pop_idx: 1,
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_0006",
    },
];
// Reachable semantic fallback for Game extcall 0011_0007 ext_0011_0007.
static AUTO_POP_0011_0007: &[ParamSpec] = &[
    ParamSpec {
        name: "action_id",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0007",
    },
    ParamSpec {
        name: "duration_ms",
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_0007",
    },
    ParamSpec {
        name: "from_value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0007",
    },
];
static AUTO_PARAMS_0011_0007: &[Param] = &[
    Param {
        name: "action_id",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0007",
    },
    Param {
        name: "duration_ms",
        pop_idx: 1,
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_0007",
    },
    Param {
        name: "from_value",
        pop_idx: 2,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0007",
    },
];
// Reachable semantic fallback for Game extcall 0011_0008 ext_0011_0008.
static AUTO_POP_0011_0008: &[ParamSpec] = &[
    ParamSpec {
        name: "action_id",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0008",
    },
    ParamSpec {
        name: "duration_ms",
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_0008",
    },
    ParamSpec {
        name: "from_value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0008",
    },
    ParamSpec {
        name: "to_value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0008",
    },
];
static AUTO_PARAMS_0011_0008: &[Param] = &[
    Param {
        name: "action_id",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0008",
    },
    Param {
        name: "duration_ms",
        pop_idx: 1,
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_0008",
    },
    Param {
        name: "from_value",
        pop_idx: 2,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0008",
    },
    Param {
        name: "to_value",
        pop_idx: 3,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0008",
    },
];
// Reachable semantic fallback for Game extcall 0011_0009 ext_0011_0009.
static AUTO_POP_0011_0009: &[ParamSpec] = &[];
static AUTO_PARAMS_0011_0009: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0011_000A ext_0011_000A.
static AUTO_POP_0011_000A: &[ParamSpec] = &[];
static AUTO_PARAMS_0011_000A: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0011_000B ext_0011_000B.
static AUTO_POP_0011_000B: &[ParamSpec] = &[ParamSpec {
    name: "action_id",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0011_000B",
}];
static AUTO_PARAMS_0011_000B: &[Param] = &[Param {
    name: "action_id",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0011_000B",
}];
// Reachable semantic fallback for Game extcall 0011_000E ext_0011_000E.
static AUTO_POP_0011_000E: &[ParamSpec] = &[
    ParamSpec {
        name: "action_id",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_000E",
    },
    ParamSpec {
        name: "duration_ms",
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_000E",
    },
    ParamSpec {
        name: "from_value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_000E",
    },
    ParamSpec {
        name: "to_value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_000E",
    },
];
static AUTO_PARAMS_0011_000E: &[Param] = &[
    Param {
        name: "action_id",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_000E",
    },
    Param {
        name: "duration_ms",
        pop_idx: 1,
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_000E",
    },
    Param {
        name: "from_value",
        pop_idx: 2,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_000E",
    },
    Param {
        name: "to_value",
        pop_idx: 3,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_000E",
    },
];
// Reachable semantic fallback for Game extcall 0011_000F ext_0011_000F.
static AUTO_POP_0011_000F: &[ParamSpec] = &[];
static AUTO_PARAMS_0011_000F: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0011_0011 ext_0011_0011.
static AUTO_POP_0011_0011: &[ParamSpec] = &[ParamSpec {
    name: "action_id",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0011_0011",
}];
static AUTO_PARAMS_0011_0011: &[Param] = &[Param {
    name: "action_id",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0011_0011",
}];
// Reachable semantic fallback for Game extcall 0011_0014 ext_0011_0014.
static AUTO_POP_0011_0014: &[ParamSpec] = &[
    ParamSpec {
        name: "action_id",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0014",
    },
    ParamSpec {
        name: "duration_ms",
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_0014",
    },
    ParamSpec {
        name: "from_value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0014",
    },
];
static AUTO_PARAMS_0011_0014: &[Param] = &[
    Param {
        name: "action_id",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0014",
    },
    Param {
        name: "duration_ms",
        pop_idx: 1,
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_0014",
    },
    Param {
        name: "from_value",
        pop_idx: 2,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_0014",
    },
];
// Reachable semantic fallback for Game extcall 0011_0015 ext_0011_0015.
static AUTO_POP_0011_0015: &[ParamSpec] = &[];
static AUTO_PARAMS_0011_0015: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0011_0017 set_active_action.
static AUTO_POP_0011_0017: &[ParamSpec] = &[ParamSpec {
    name: "action_id",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for set_active_action",
}];
static AUTO_PARAMS_0011_0017: &[Param] = &[Param {
    name: "action_id",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for set_active_action",
}];
// Reachable semantic fallback for Game extcall 0011_0018 get_active_action.
static AUTO_POP_0011_0018: &[ParamSpec] = &[ParamSpec {
    name: "action_id",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for get_active_action",
}];
static AUTO_PARAMS_0011_0018: &[Param] = &[Param {
    name: "action_id",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for get_active_action",
}];
// Reachable semantic fallback for Game extcall 0011_001C action_uninit.
static AUTO_POP_0011_001C: &[ParamSpec] = &[];
static AUTO_PARAMS_0011_001C: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0011_001D ext_0011_001D.
static AUTO_POP_0011_001D: &[ParamSpec] = &[
    ParamSpec {
        name: "action_id",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_001D",
    },
    ParamSpec {
        name: "duration_ms",
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_001D",
    },
    ParamSpec {
        name: "from_value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_001D",
    },
    ParamSpec {
        name: "to_value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_001D",
    },
];
static AUTO_PARAMS_0011_001D: &[Param] = &[
    Param {
        name: "action_id",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_001D",
    },
    Param {
        name: "duration_ms",
        pop_idx: 1,
        kind: ParamKind::DurationMs,
        meaning: "runtime pop-order parameter for ext_0011_001D",
    },
    Param {
        name: "from_value",
        pop_idx: 2,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_001D",
    },
    Param {
        name: "to_value",
        pop_idx: 3,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0011_001D",
    },
];
// Reachable semantic fallback for Game extcall 0011_001E set_action_clear.
static AUTO_POP_0011_001E: &[ParamSpec] = &[];
static AUTO_PARAMS_0011_001E: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0001 app_exec.
static AUTO_POP_0012_0001: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0001: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0003 ext_0012_0003.
static AUTO_POP_0012_0003: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0003: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0004 ext_0012_0004.
static AUTO_POP_0012_0004: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0004: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0005 ext_0012_0005.
static AUTO_POP_0012_0005: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0005: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0006 ext_0012_0006.
static AUTO_POP_0012_0006: &[ParamSpec] = &[
    ParamSpec {
        name: "value",
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0012_0006",
    },
    ParamSpec {
        name: "mode",
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_0012_0006",
    },
];
static AUTO_PARAMS_0012_0006: &[Param] = &[
    Param {
        name: "value",
        pop_idx: 0,
        kind: ParamKind::Integer,
        meaning: "runtime pop-order parameter for ext_0012_0006",
    },
    Param {
        name: "mode",
        pop_idx: 1,
        kind: ParamKind::Mode,
        meaning: "runtime pop-order parameter for ext_0012_0006",
    },
];
// Reachable semantic fallback for Game extcall 0012_0007 ext_0012_0007.
static AUTO_POP_0012_0007: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0007: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0008 file_exist.
static AUTO_POP_0012_0008: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0008: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0009 wsprint.
static AUTO_POP_0012_0009: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0009: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_000A check_disc.
static AUTO_POP_0012_000A: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_000A: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_000C ext_0012_000C.
static AUTO_POP_0012_000C: &[ParamSpec] = &[ParamSpec {
    name: "value",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0012_000C",
}];
static AUTO_PARAMS_0012_000C: &[Param] = &[Param {
    name: "value",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0012_000C",
}];
// Reachable semantic fallback for Game extcall 0012_000D ext_0012_000D.
static AUTO_POP_0012_000D: &[ParamSpec] = &[ParamSpec {
    name: "value",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0012_000D",
}];
static AUTO_PARAMS_0012_000D: &[Param] = &[Param {
    name: "value",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0012_000D",
}];
// Reachable semantic fallback for Game extcall 0012_000E update_access.
static AUTO_POP_0012_000E: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_000E: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_000F ext_0012_000F.
static AUTO_POP_0012_000F: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_000F: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0011 ext_0012_0011.
static AUTO_POP_0012_0011: &[ParamSpec] = &[ParamSpec {
    name: "value",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0012_0011",
}];
static AUTO_PARAMS_0012_0011: &[Param] = &[Param {
    name: "value",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0012_0011",
}];
// Reachable semantic fallback for Game extcall 0012_0012 ext_0012_0012.
static AUTO_POP_0012_0012: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0012: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0015 ext_0012_0015.
static AUTO_POP_0012_0015: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0015: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_001C ext_0012_001C.
static AUTO_POP_0012_001C: &[ParamSpec] = &[ParamSpec {
    name: "value",
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0012_001C",
}];
static AUTO_PARAMS_0012_001C: &[Param] = &[Param {
    name: "value",
    pop_idx: 0,
    kind: ParamKind::Integer,
    meaning: "runtime pop-order parameter for ext_0012_001C",
}];
// Reachable semantic fallback for Game extcall 0012_001D ext_0012_001D.
static AUTO_POP_0012_001D: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_001D: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0020 close_file_not_handle.
static AUTO_POP_0012_0020: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0020: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0022 file_string.
static AUTO_POP_0012_0022: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0022: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0023 set_last_process.
static AUTO_POP_0012_0023: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0023: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0012_0028 ext_0012_0028.
static AUTO_POP_0012_0028: &[ParamSpec] = &[];
static AUTO_PARAMS_0012_0028: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0014_0000 random.
static AUTO_POP_0014_0000: &[ParamSpec] = &[];
static AUTO_PARAMS_0014_0000: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0015_0000 create_thread.
static AUTO_POP_0015_0000: &[ParamSpec] = &[];
static AUTO_PARAMS_0015_0000: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0015_0002 ext_0015_0002.
static AUTO_POP_0015_0002: &[ParamSpec] = &[];
static AUTO_PARAMS_0015_0002: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0017_0000 create_message.
static AUTO_POP_0017_0000: &[ParamSpec] = &[];
static AUTO_PARAMS_0017_0000: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0017_0001 get_message.
static AUTO_POP_0017_0001: &[ParamSpec] = &[];
static AUTO_PARAMS_0017_0001: &[Param] = &[];
// Reachable semantic fallback for Game extcall 0017_0002 get_message_param.
static AUTO_POP_0017_0002: &[ParamSpec] = &[];
static AUTO_PARAMS_0017_0002: &[Param] = &[];

static AUTO_REACHABLE_SIGNATURES: &[ExtSig] = &[
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0000 text_init; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 0, canonical_name: "text_init", name: "text_init",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 8, pop_order: AUTO_POP_0002_0000, display_order: ORDER_8, params: AUTO_PARAMS_0002_0000,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0000 (text_init) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_init" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0000" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0001 text_set_icon; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 1, canonical_name: "text_set_icon", name: "text_set_icon",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 4, pop_order: AUTO_POP_0002_0001, display_order: ORDER_4, params: AUTO_PARAMS_0002_0001,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0001 (text_set_icon) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_set_icon" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0001" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0005 text_set_btn; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 5, canonical_name: "text_set_btn", name: "text_set_btn",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_0005, display_order: ORDER_0, params: AUTO_PARAMS_0002_0005,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0005 (text_set_btn) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_set_btn" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0005" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0009 ext_0002_0009; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 9, canonical_name: "ext_0002_0009", name: "ext_0002_0009",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_0009, display_order: ORDER_0, params: AUTO_PARAMS_0002_0009,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0009 (ext_0002_0009) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable ext_0002_0009" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0009" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_000A text_get_time; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 10, canonical_name: "text_get_time", name: "text_get_time",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0002_000A, display_order: ORDER_1, params: AUTO_PARAMS_0002_000A,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:000A (text_get_time) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_get_time" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:000A" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_000B text_window_set_alpha; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 11, canonical_name: "text_window_set_alpha", name: "text_window_set_alpha",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0002_000B, display_order: ORDER_2, params: AUTO_PARAMS_0002_000B,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:000B (text_window_set_alpha) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_window_set_alpha" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:000B" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_000C text_voice_play; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 12, canonical_name: "text_voice_play", name: "text_voice_play",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0002_000C, display_order: ORDER_1, params: AUTO_PARAMS_0002_000C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:000C (text_voice_play) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_voice_play" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:000C" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_000D ext_0002_000D; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 13, canonical_name: "ext_0002_000D", name: "ext_0002_000D",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_000D, display_order: ORDER_0, params: AUTO_PARAMS_0002_000D,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:000D (ext_0002_000D) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable ext_0002_000D" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:000D" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_000E text_set_icon_animation_time; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 14, canonical_name: "text_set_icon_animation_time", name: "text_set_icon_animation_time",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0002_000E, display_order: ORDER_2, params: AUTO_PARAMS_0002_000E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:000E (text_set_icon_animation_time) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_set_icon_animation_time" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:000E" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0012 text_n; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 18, canonical_name: "text_n", name: "text_n",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_0012, display_order: ORDER_0, params: AUTO_PARAMS_0002_0012,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0012 (text_n) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_n" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0012" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0013 text_cat; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 19, canonical_name: "text_cat", name: "text_cat",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_0013, display_order: ORDER_0, params: AUTO_PARAMS_0002_0013,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0013 (text_cat) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_cat" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0013" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0014 set_history; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 20, canonical_name: "set_history", name: "set_history",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_0014, display_order: ORDER_0, params: AUTO_PARAMS_0002_0014,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0014 (set_history) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable set_history" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0014" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0015 is_text_visible; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 21, canonical_name: "is_text_visible", name: "is_text_visible",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_0015, display_order: ORDER_0, params: AUTO_PARAMS_0002_0015,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0015 (is_text_visible) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable is_text_visible" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0015" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0016 text_set_base; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 22, canonical_name: "text_set_base", name: "text_set_base",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_0016, display_order: ORDER_0, params: AUTO_PARAMS_0002_0016,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0016 (text_set_base) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_set_base" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0016" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0017 enable_voice_cut; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 23, canonical_name: "enable_voice_cut", name: "enable_voice_cut",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_0017, display_order: ORDER_0, params: AUTO_PARAMS_0002_0017,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0017 (enable_voice_cut) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable enable_voice_cut" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0017" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0018 is_voice_cut; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 24, canonical_name: "is_voice_cut", name: "is_voice_cut",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_0018, display_order: ORDER_0, params: AUTO_PARAMS_0002_0018,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0018 (is_voice_cut) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable is_voice_cut" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0018" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0019 texttimecheckset; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 25, canonical_name: "texttimecheckset", name: "texttimecheckset",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_0019, display_order: ORDER_0, params: AUTO_PARAMS_0002_0019,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0019 (texttimecheckset) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable texttimecheckset" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0019" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_001A ext_0002_001A; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 26, canonical_name: "ext_0002_001A", name: "ext_0002_001A",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_001A, display_order: ORDER_0, params: AUTO_PARAMS_0002_001A,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:001A (ext_0002_001A) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable ext_0002_001A" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:001A" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_001B ext_0002_001B; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 27, canonical_name: "ext_0002_001B", name: "ext_0002_001B",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_001B, display_order: ORDER_0, params: AUTO_PARAMS_0002_001B,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:001B (ext_0002_001B) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable ext_0002_001B" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:001B" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_001D textredraw; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 29, canonical_name: "textredraw", name: "textredraw",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_001D, display_order: ORDER_0, params: AUTO_PARAMS_0002_001D,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:001D (textredraw) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable textredraw" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:001D" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_001E set_text_mode; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 30, canonical_name: "set_text_mode", name: "set_text_mode",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_001E, display_order: ORDER_0, params: AUTO_PARAMS_0002_001E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:001E (set_text_mode) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable set_text_mode" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:001E" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_001F text_init_visualnovelmode; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 31, canonical_name: "text_init_visualnovelmode", name: "text_init_visualnovelmode",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0002_001F, display_order: ORDER_0, params: AUTO_PARAMS_0002_001F,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:001F (text_init_visualnovelmode) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_init_visualnovelmode" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:001F" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0002_0020 text_set_icon_mode; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 2, index: 32, canonical_name: "text_set_icon_mode", name: "text_set_icon_mode",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0002_0020, display_order: ORDER_1, params: AUTO_PARAMS_0002_0020,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesTextState],
        purpose: "Reachable Game.exe extcall 0002:0020 (text_set_icon_mode) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 2 extcall handler checked for reachable text_set_icon_mode" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0002:0020" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0007 set_priority; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 7, canonical_name: "set_priority", name: "set_priority",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0003_0007, display_order: ORDER_1, params: AUTO_PARAMS_0003_0007,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0007 (set_priority) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable set_priority" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0007" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable set_priority; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0008 ext_0003_0008; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 8, canonical_name: "ext_0003_0008", name: "ext_0003_0008",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0003_0008, display_order: ORDER_1, params: AUTO_PARAMS_0003_0008,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0008 (ext_0003_0008) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable ext_0003_0008" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0008" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0003_0008; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0009 sp_set_center; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 9, canonical_name: "sp_set_center", name: "sp_set_center",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0009, display_order: ORDER_0, params: AUTO_PARAMS_0003_0009,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0009 (sp_set_center) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_set_center" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0009" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_set_center; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_000B sp_cls_ex; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 11, canonical_name: "sp_cls_ex", name: "sp_cls_ex",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0003_000B, display_order: ORDER_1, params: AUTO_PARAMS_0003_000B,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:000B (sp_cls_ex) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_cls_ex" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:000B" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_cls_ex; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_000C set_filter; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 12, canonical_name: "set_filter", name: "set_filter",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0003_000C, display_order: ORDER_2, params: AUTO_PARAMS_0003_000C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:000C (set_filter) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable set_filter" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:000C" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable set_filter; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_000D sp_cls_transition; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 13, canonical_name: "sp_cls_transition", name: "sp_cls_transition",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0003_000D, display_order: ORDER_2, params: AUTO_PARAMS_0003_000D,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:000D (sp_cls_transition) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_cls_transition" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:000D" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_cls_transition; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_000E sp_move_ex; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 14, canonical_name: "sp_move_ex", name: "sp_move_ex",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 4, pop_order: AUTO_POP_0003_000E, display_order: ORDER_4, params: AUTO_PARAMS_0003_000E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:000E (sp_move_ex) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_move_ex" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:000E" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_move_ex; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_000F sp_set_rect_pos; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 15, canonical_name: "sp_set_rect_pos", name: "sp_set_rect_pos",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_000F, display_order: ORDER_0, params: AUTO_PARAMS_0003_000F,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:000F (sp_set_rect_pos) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_set_rect_pos" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:000F" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_set_rect_pos; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0010 ext_0003_0010; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 16, canonical_name: "ext_0003_0010", name: "ext_0003_0010",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0010, display_order: ORDER_0, params: AUTO_PARAMS_0003_0010,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0010 (ext_0003_0010) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable ext_0003_0010" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0010" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0003_0010; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0012 sp_set_rotate; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 18, canonical_name: "sp_set_rotate", name: "sp_set_rotate",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0003_0012, display_order: ORDER_2, params: AUTO_PARAMS_0003_0012,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0012 (sp_set_rotate) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_set_rotate" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0012" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_set_rotate; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0013 face_init; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 19, canonical_name: "face_init", name: "face_init",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0013, display_order: ORDER_0, params: AUTO_PARAMS_0003_0013,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0013 (face_init) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable face_init" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0013" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable face_init; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0014 face_set; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 20, canonical_name: "face_set", name: "face_set",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0014, display_order: ORDER_0, params: AUTO_PARAMS_0003_0014,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0014 (face_set) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable face_set" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0014" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable face_set; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0015 not_image_sp_get_color; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 21, canonical_name: "not_image_sp_get_color", name: "not_image_sp_get_color",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0015, display_order: ORDER_0, params: AUTO_PARAMS_0003_0015,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0015 (not_image_sp_get_color) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable not_image_sp_get_color" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0015" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable not_image_sp_get_color; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0017 face_cls; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 23, canonical_name: "face_cls", name: "face_cls",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0017, display_order: ORDER_0, params: AUTO_PARAMS_0003_0017,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0017 (face_cls) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable face_cls" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0017" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable face_cls; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0018 sp_set_rect; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 24, canonical_name: "sp_set_rect", name: "sp_set_rect",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 3, pop_order: AUTO_POP_0003_0018, display_order: ORDER_3, params: AUTO_PARAMS_0003_0018,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0018 (sp_set_rect) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_set_rect" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0018" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_set_rect; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_001A not_image_sp_get_alpha; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 26, canonical_name: "not_image_sp_get_alpha", name: "not_image_sp_get_alpha",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_001A, display_order: ORDER_0, params: AUTO_PARAMS_0003_001A,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:001A (not_image_sp_get_alpha) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable not_image_sp_get_alpha" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:001A" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable not_image_sp_get_alpha; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_001B not_image_sp_get_rotate; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 27, canonical_name: "not_image_sp_get_rotate", name: "not_image_sp_get_rotate",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_001B, display_order: ORDER_0, params: AUTO_PARAMS_0003_001B,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:001B (not_image_sp_get_rotate) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable not_image_sp_get_rotate" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:001B" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable not_image_sp_get_rotate; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_001C ext_0003_001C; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 28, canonical_name: "ext_0003_001C", name: "ext_0003_001C",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 4, pop_order: AUTO_POP_0003_001C, display_order: ORDER_4, params: AUTO_PARAMS_0003_001C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:001C (ext_0003_001C) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable ext_0003_001C" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:001C" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0003_001C; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_001D ext_0003_001D; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 29, canonical_name: "ext_0003_001D", name: "ext_0003_001D",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0003_001D, display_order: ORDER_1, params: AUTO_PARAMS_0003_001D,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:001D (ext_0003_001D) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable ext_0003_001D" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:001D" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0003_001D; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_001E ext_0003_001E; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 30, canonical_name: "ext_0003_001E", name: "ext_0003_001E",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0003_001E, display_order: ORDER_1, params: AUTO_PARAMS_0003_001E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:001E (ext_0003_001E) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable ext_0003_001E" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:001E" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0003_001E; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_001F ext_0003_001F; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 31, canonical_name: "ext_0003_001F", name: "ext_0003_001F",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_001F, display_order: ORDER_0, params: AUTO_PARAMS_0003_001F,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:001F (ext_0003_001F) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable ext_0003_001F" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:001F" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0003_001F; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0020 sp_create; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 32, canonical_name: "sp_create", name: "sp_create",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0003_0020, display_order: ORDER_1, params: AUTO_PARAMS_0003_0020,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0020 (sp_create) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_create" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0020" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_create; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0022 ext_0003_0022; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 34, canonical_name: "ext_0003_0022", name: "ext_0003_0022",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0022, display_order: ORDER_0, params: AUTO_PARAMS_0003_0022,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0022 (ext_0003_0022) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable ext_0003_0022" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0022" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0003_0022; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0023 ext_0003_0023; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 35, canonical_name: "ext_0003_0023", name: "ext_0003_0023",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0003_0023, display_order: ORDER_2, params: AUTO_PARAMS_0003_0023,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0023 (ext_0003_0023) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable ext_0003_0023" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0023" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0003_0023; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0024 not_image_sp_get_scale; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 36, canonical_name: "not_image_sp_get_scale", name: "not_image_sp_get_scale",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0003_0024, display_order: ORDER_1, params: AUTO_PARAMS_0003_0024,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0024 (not_image_sp_get_scale) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable not_image_sp_get_scale" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0024" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable not_image_sp_get_scale; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0025 sp_set_color_0x; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 37, canonical_name: "sp_set_color_0x", name: "sp_set_color_0x",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0025, display_order: ORDER_0, params: AUTO_PARAMS_0003_0025,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0025 (sp_set_color_0x) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_set_color_0x" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0025" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_set_color_0x; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0026 sp_bitblt; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 38, canonical_name: "sp_bitblt", name: "sp_bitblt",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0003_0026, display_order: ORDER_2, params: AUTO_PARAMS_0003_0026,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0026 (sp_bitblt) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_bitblt" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0026" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_bitblt; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0027 sp_set_shake; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 39, canonical_name: "sp_set_shake", name: "sp_set_shake",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0027, display_order: ORDER_0, params: AUTO_PARAMS_0003_0027,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0027 (sp_set_shake) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_set_shake" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0027" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_set_shake; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0028 sp_paint; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 40, canonical_name: "sp_paint", name: "sp_paint",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0028, display_order: ORDER_0, params: AUTO_PARAMS_0003_0028,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0028 (sp_paint) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_paint" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0028" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_paint; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_002C sp_wait_draw; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 44, canonical_name: "sp_wait_draw", name: "sp_wait_draw",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_002C, display_order: ORDER_0, params: AUTO_PARAMS_0003_002C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:002C (sp_wait_draw) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_wait_draw" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:002C" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_wait_draw; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_002E sp_show; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 46, canonical_name: "sp_show", name: "sp_show",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_002E, display_order: ORDER_0, params: AUTO_PARAMS_0003_002E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:002E (sp_show) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_show" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:002E" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_show; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_002F sp_hide; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 47, canonical_name: "sp_hide", name: "sp_hide",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0003_002F, display_order: ORDER_1, params: AUTO_PARAMS_0003_002F,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:002F (sp_hide) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_hide" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:002F" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_hide; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0030 ext_0003_0030; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 48, canonical_name: "ext_0003_0030", name: "ext_0003_0030",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0030, display_order: ORDER_0, params: AUTO_PARAMS_0003_0030,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0030 (ext_0003_0030) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable ext_0003_0030" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0030" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0003_0030; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0031 sp_set_child; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 49, canonical_name: "sp_set_child", name: "sp_set_child",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0031, display_order: ORDER_0, params: AUTO_PARAMS_0003_0031,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0031 (sp_set_child) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_set_child" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0031" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_set_child; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0032 sp_set_transition; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 50, canonical_name: "sp_set_transition", name: "sp_set_transition",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 4, pop_order: AUTO_POP_0003_0032, display_order: ORDER_4, params: AUTO_PARAMS_0003_0032,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0032 (sp_set_transition) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_set_transition" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0032" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_set_transition; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0033 sp_copy_image; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 51, canonical_name: "sp_copy_image", name: "sp_copy_image",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0003_0033, display_order: ORDER_1, params: AUTO_PARAMS_0003_0033,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0033 (sp_copy_image) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_copy_image" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0033" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_copy_image; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0034 sp_transition; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 52, canonical_name: "sp_transition", name: "sp_transition",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0034, display_order: ORDER_0, params: AUTO_PARAMS_0003_0034,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0034 (sp_transition) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_transition" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0034" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_transition; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0035 set_aspect_position_type; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 53, canonical_name: "set_aspect_position_type", name: "set_aspect_position_type",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0035, display_order: ORDER_0, params: AUTO_PARAMS_0003_0035,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0035 (set_aspect_position_type) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable set_aspect_position_type" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0035" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable set_aspect_position_type; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0036 get_backbuffer; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 54, canonical_name: "get_backbuffer", name: "get_backbuffer",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0036, display_order: ORDER_0, params: AUTO_PARAMS_0003_0036,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0036 (get_backbuffer) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable get_backbuffer" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0036" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable get_backbuffer; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0037 sp_set_mask; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 55, canonical_name: "sp_set_mask", name: "sp_set_mask",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0003_0037, display_order: ORDER_0, params: AUTO_PARAMS_0003_0037,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0037 (sp_set_mask) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable sp_set_mask" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0037" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable sp_set_mask; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0003_0038 ext_0003_0038; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 3, index: 56, canonical_name: "ext_0003_0038", name: "ext_0003_0038",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 3, pop_order: AUTO_POP_0003_0038, display_order: ORDER_3, params: AUTO_PARAMS_0003_0038,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::MutatesSprite],
        purpose: "Reachable Game.exe extcall 0003:0038 (ext_0003_0038) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 3 extcall handler checked for reachable ext_0003_0038" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0003:0038" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0003_0038; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0004_0004 bgm_get_auto_volume; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 4, index: 4, canonical_name: "bgm_get_auto_volume", name: "bgm_get_auto_volume",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0004_0004, display_order: ORDER_0, params: AUTO_PARAMS_0004_0004,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 0004:0004 (bgm_get_auto_volume) for BGM audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 4 extcall handler checked for reachable bgm_get_auto_volume" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0004:0004" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable bgm_get_auto_volume; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0004_0006 bgm_set_auto_volume; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 4, index: 6, canonical_name: "bgm_set_auto_volume", name: "bgm_set_auto_volume",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0004_0006, display_order: ORDER_0, params: AUTO_PARAMS_0004_0006,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 0004:0006 (bgm_set_auto_volume) for BGM audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 4 extcall handler checked for reachable bgm_set_auto_volume" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0004:0006" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable bgm_set_auto_volume; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0004_0008 get_bgm_filename; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 4, index: 8, canonical_name: "get_bgm_filename", name: "get_bgm_filename",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0004_0008, display_order: ORDER_0, params: AUTO_PARAMS_0004_0008,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 0004:0008 (get_bgm_filename) for BGM audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 4 extcall handler checked for reachable get_bgm_filename" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0004:0008" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable get_bgm_filename; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0004_000B set_master_volume; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 4, index: 11, canonical_name: "set_master_volume", name: "set_master_volume",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0004_000B, display_order: ORDER_0, params: AUTO_PARAMS_0004_000B,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 0004:000B (set_master_volume) for BGM audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 4 extcall handler checked for reachable set_master_volume" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0004:000B" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable set_master_volume; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0004_000C get_master_volume; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 4, index: 12, canonical_name: "get_master_volume", name: "get_master_volume",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0004_000C, display_order: ORDER_0, params: AUTO_PARAMS_0004_000C,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 0004:000C (get_master_volume) for BGM audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 4 extcall handler checked for reachable get_master_volume" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0004:000C" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable get_master_volume; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0004_000D mute_master_volume; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 4, index: 13, canonical_name: "mute_master_volume", name: "mute_master_volume",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0004_000D, display_order: ORDER_0, params: AUTO_PARAMS_0004_000D,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 0004:000D (mute_master_volume) for BGM audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 4 extcall handler checked for reachable mute_master_volume" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0004:000D" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable mute_master_volume; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0004_000E bgm_mute; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 4, index: 14, canonical_name: "bgm_mute", name: "bgm_mute",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0004_000E, display_order: ORDER_0, params: AUTO_PARAMS_0004_000E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 0004:000E (bgm_mute) for BGM audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 4 extcall handler checked for reachable bgm_mute" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0004:000E" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable bgm_mute; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0004_000F mute_bgm_auto_volume; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 4, index: 15, canonical_name: "mute_bgm_auto_volume", name: "mute_bgm_auto_volume",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0004_000F, display_order: ORDER_0, params: AUTO_PARAMS_0004_000F,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 0004:000F (mute_bgm_auto_volume) for BGM audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 4 extcall handler checked for reachable mute_bgm_auto_volume" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0004:000F" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable mute_bgm_auto_volume; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0005_0006 se_unload; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 5, index: 6, canonical_name: "se_unload", name: "se_unload",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0005_0006, display_order: ORDER_0, params: AUTO_PARAMS_0005_0006,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 0005:0006 (se_unload) for sound-effect audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 5 extcall handler checked for reachable se_unload" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0005:0006" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable se_unload; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0005_0007 se_wait; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 5, index: 7, canonical_name: "se_wait", name: "se_wait",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0005_0007, display_order: ORDER_0, params: AUTO_PARAMS_0005_0007,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 0005:0007 (se_wait) for sound-effect audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 5 extcall handler checked for reachable se_wait" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0005:0007" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable se_wait; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0005_000E se_mute; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 5, index: 14, canonical_name: "se_mute", name: "se_mute",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0005_000E, display_order: ORDER_0, params: AUTO_PARAMS_0005_000E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 0005:000E (se_mute) for sound-effect audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 5 extcall handler checked for reachable se_mute" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0005:000E" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable se_mute; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0006_0000 select_init; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 6, index: 0, canonical_name: "select_init", name: "select_init",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0006_0000, display_order: ORDER_1, params: AUTO_PARAMS_0006_0000,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0006:0000 (select_init) for select/menu state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 6 extcall handler checked for reachable select_init" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0006:0000" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0006_0001 select; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 6, index: 1, canonical_name: "select", name: "select",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0006_0001, display_order: ORDER_0, params: AUTO_PARAMS_0006_0001,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0006:0001 (select) for select/menu state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 6 extcall handler checked for reachable select" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0006:0001" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0006_0002 ext_0006_0002; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 6, index: 2, canonical_name: "ext_0006_0002", name: "ext_0006_0002",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0006_0002, display_order: ORDER_0, params: AUTO_PARAMS_0006_0002,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0006:0002 (ext_0006_0002) for select/menu state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 6 extcall handler checked for reachable ext_0006_0002" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0006:0002" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0006_0003 ext_0006_0003; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 6, index: 3, canonical_name: "ext_0006_0003", name: "ext_0006_0003",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0006_0003, display_order: ORDER_0, params: AUTO_PARAMS_0006_0003,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0006:0003 (ext_0006_0003) for select/menu state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 6 extcall handler checked for reachable ext_0006_0003" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0006:0003" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0006_0004 select_clear; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 6, index: 4, canonical_name: "select_clear", name: "select_clear",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0006_0004, display_order: ORDER_0, params: AUTO_PARAMS_0006_0004,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0006:0004 (select_clear) for select/menu state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 6 extcall handler checked for reachable select_clear" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0006:0004" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0006_0006 select_set_process; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 6, index: 6, canonical_name: "select_set_process", name: "select_set_process",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0006_0006, display_order: ORDER_0, params: AUTO_PARAMS_0006_0006,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0006:0006 (select_set_process) for select/menu state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 6 extcall handler checked for reachable select_set_process" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0006:0006" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0007_0006 wait_clear; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 7, index: 6, canonical_name: "wait_clear", name: "wait_clear",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0007_0006, display_order: ORDER_0, params: AUTO_PARAMS_0007_0006,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0007:0006 (wait_clear) for wait/task scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 7 extcall handler checked for reachable wait_clear" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0007:0006" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_0004 btn_hide; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 4, canonical_name: "btn_hide", name: "btn_hide",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_0004, display_order: ORDER_0, params: AUTO_PARAMS_0008_0004,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:0004 (btn_hide) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable btn_hide" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:0004" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable btn_hide; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_0005 btn_show; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 5, canonical_name: "btn_show", name: "btn_show",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_0005, display_order: ORDER_0, params: AUTO_PARAMS_0008_0005,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:0005 (btn_show) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable btn_show" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:0005" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable btn_show; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_0009 ext_0008_0009; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 9, canonical_name: "ext_0008_0009", name: "ext_0008_0009",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_0009, display_order: ORDER_0, params: AUTO_PARAMS_0008_0009,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:0009 (ext_0008_0009) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable ext_0008_0009" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:0009" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0008_0009; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_000A ext_0008_000A; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 10, canonical_name: "ext_0008_000A", name: "ext_0008_000A",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_000A, display_order: ORDER_0, params: AUTO_PARAMS_0008_000A,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:000A (ext_0008_000A) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable ext_0008_000A" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:000A" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0008_000A; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_000B ext_0008_000B; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 11, canonical_name: "ext_0008_000B", name: "ext_0008_000B",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 3, pop_order: AUTO_POP_0008_000B, display_order: ORDER_3, params: AUTO_PARAMS_0008_000B,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:000B (ext_0008_000B) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable ext_0008_000B" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:000B" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0008_000B; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_000C ext_0008_000C; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 12, canonical_name: "ext_0008_000C", name: "ext_0008_000C",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_000C, display_order: ORDER_0, params: AUTO_PARAMS_0008_000C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:000C (ext_0008_000C) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable ext_0008_000C" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:000C" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0008_000C; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_000D btn_set_toggle; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 13, canonical_name: "btn_set_toggle", name: "btn_set_toggle",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0008_000D, display_order: ORDER_2, params: AUTO_PARAMS_0008_000D,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:000D (btn_set_toggle) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable btn_set_toggle" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:000D" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable btn_set_toggle; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_000E ext_0008_000E; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 14, canonical_name: "ext_0008_000E", name: "ext_0008_000E",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 3, pop_order: AUTO_POP_0008_000E, display_order: ORDER_3, params: AUTO_PARAMS_0008_000E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:000E (ext_0008_000E) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable ext_0008_000E" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:000E" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0008_000E; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_000F btn_enable; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 15, canonical_name: "btn_enable", name: "btn_enable",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0008_000F, display_order: ORDER_1, params: AUTO_PARAMS_0008_000F,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:000F (btn_enable) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable btn_enable" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:000F" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable btn_enable; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_0010 btn_set_alpha_0x; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 16, canonical_name: "btn_set_alpha_0x", name: "btn_set_alpha_0x",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_0010, display_order: ORDER_0, params: AUTO_PARAMS_0008_0010,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:0010 (btn_set_alpha_0x) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable btn_set_alpha_0x" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:0010" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable btn_set_alpha_0x; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_0012 error_btn_expansion; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 18, canonical_name: "error_btn_expansion", name: "error_btn_expansion",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_0012, display_order: ORDER_0, params: AUTO_PARAMS_0008_0012,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:0012 (error_btn_expansion) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable error_btn_expansion" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:0012" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable error_btn_expansion; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_0015 btn_set_anim; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 21, canonical_name: "btn_set_anim", name: "btn_set_anim",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_0015, display_order: ORDER_0, params: AUTO_PARAMS_0008_0015,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:0015 (btn_set_anim) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable btn_set_anim" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:0015" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable btn_set_anim; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_0016 btn_set_hit; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 22, canonical_name: "btn_set_hit", name: "btn_set_hit",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_0016, display_order: ORDER_0, params: AUTO_PARAMS_0008_0016,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:0016 (btn_set_hit) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable btn_set_hit" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:0016" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable btn_set_hit; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_0017 btn_get_onmouse; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 23, canonical_name: "btn_get_onmouse", name: "btn_get_onmouse",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_0017, display_order: ORDER_0, params: AUTO_PARAMS_0008_0017,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:0017 (btn_get_onmouse) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable btn_get_onmouse" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:0017" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable btn_get_onmouse; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_0018 btn_anim_clear; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 24, canonical_name: "btn_anim_clear", name: "btn_anim_clear",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_0018, display_order: ORDER_0, params: AUTO_PARAMS_0008_0018,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:0018 (btn_anim_clear) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable btn_anim_clear" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:0018" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable btn_anim_clear; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0008_001A btn_onmouse_clear; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 8, index: 26, canonical_name: "btn_onmouse_clear", name: "btn_onmouse_clear",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0008_001A, display_order: ORDER_0, params: AUTO_PARAMS_0008_001A,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 0008:001A (btn_onmouse_clear) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 8 extcall handler checked for reachable btn_onmouse_clear" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0008:001A" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable btn_onmouse_clear; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0000 skip_set; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 0, canonical_name: "skip_set", name: "skip_set",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0000, display_order: ORDER_0, params: AUTO_PARAMS_0009_0000,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0000 (skip_set) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable skip_set" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0000" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0001 skip_is; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 1, canonical_name: "skip_is", name: "skip_is",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0001, display_order: ORDER_0, params: AUTO_PARAMS_0009_0001,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0001 (skip_is) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable skip_is" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0001" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0002 auto_set; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 2, canonical_name: "auto_set", name: "auto_set",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0002, display_order: ORDER_0, params: AUTO_PARAMS_0009_0002,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0002 (auto_set) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable auto_set" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0002" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0003 auto_is; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 3, canonical_name: "auto_is", name: "auto_is",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0003, display_order: ORDER_0, params: AUTO_PARAMS_0009_0003,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0003 (auto_is) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable auto_is" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0003" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0004 ext_0009_0004; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 4, canonical_name: "ext_0009_0004", name: "ext_0009_0004",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0004, display_order: ORDER_0, params: AUTO_PARAMS_0009_0004,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0004 (ext_0009_0004) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0004" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0004" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0005 auto_get_time; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 5, canonical_name: "auto_get_time", name: "auto_get_time",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0005, display_order: ORDER_0, params: AUTO_PARAMS_0009_0005,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0005 (auto_get_time) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable auto_get_time" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0005" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0006 ext_0009_0006; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 6, canonical_name: "ext_0009_0006", name: "ext_0009_0006",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0006, display_order: ORDER_0, params: AUTO_PARAMS_0009_0006,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0006 (ext_0009_0006) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0006" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0006" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0007 ext_0009_0007; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 7, canonical_name: "ext_0009_0007", name: "ext_0009_0007",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0007, display_order: ORDER_0, params: AUTO_PARAMS_0009_0007,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0007 (ext_0009_0007) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0007" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0007" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0008 ext_0009_0008; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 8, canonical_name: "ext_0009_0008", name: "ext_0009_0008",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0008, display_order: ORDER_0, params: AUTO_PARAMS_0009_0008,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0008 (ext_0009_0008) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0008" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0008" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0009 ext_0009_0009; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 9, canonical_name: "ext_0009_0009", name: "ext_0009_0009",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0009_0009, display_order: ORDER_1, params: AUTO_PARAMS_0009_0009,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0009 (ext_0009_0009) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0009" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0009" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_000A ext_0009_000A; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 10, canonical_name: "ext_0009_000A", name: "ext_0009_000A",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_000A, display_order: ORDER_0, params: AUTO_PARAMS_0009_000A,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:000A (ext_0009_000A) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_000A" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:000A" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_000C ext_0009_000C; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 12, canonical_name: "ext_0009_000C", name: "ext_0009_000C",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0009_000C, display_order: ORDER_2, params: AUTO_PARAMS_0009_000C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:000C (ext_0009_000C) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_000C" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:000C" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_000E ext_0009_000E; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 14, canonical_name: "ext_0009_000E", name: "ext_0009_000E",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_000E, display_order: ORDER_0, params: AUTO_PARAMS_0009_000E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:000E (ext_0009_000E) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_000E" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:000E" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_000F load_font; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 15, canonical_name: "load_font", name: "load_font",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_000F, display_order: ORDER_0, params: AUTO_PARAMS_0009_000F,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:000F (load_font) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable load_font" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:000F" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0011 set_language; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 17, canonical_name: "set_language", name: "set_language",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0011, display_order: ORDER_0, params: AUTO_PARAMS_0009_0011,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0011 (set_language) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable set_language" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0011" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0012 key_canncel; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 18, canonical_name: "key_canncel", name: "key_canncel",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0009_0012, display_order: ORDER_2, params: AUTO_PARAMS_0009_0012,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0012 (key_canncel) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable key_canncel" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0012" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0013 set_font_color; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 19, canonical_name: "set_font_color", name: "set_font_color",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0013, display_order: ORDER_0, params: AUTO_PARAMS_0009_0013,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0013 (set_font_color) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable set_font_color" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0013" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0014 load_font_ex; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 20, canonical_name: "load_font_ex", name: "load_font_ex",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0014, display_order: ORDER_0, params: AUTO_PARAMS_0009_0014,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0014 (load_font_ex) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable load_font_ex" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0014" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0015 ext_0009_0015; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 21, canonical_name: "ext_0009_0015", name: "ext_0009_0015",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0015, display_order: ORDER_0, params: AUTO_PARAMS_0009_0015,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0015 (ext_0009_0015) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0015" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0015" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0016 ext_0009_0016; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 22, canonical_name: "ext_0009_0016", name: "ext_0009_0016",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0016, display_order: ORDER_0, params: AUTO_PARAMS_0009_0016,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0016 (ext_0009_0016) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0016" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0016" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0017 ext_0009_0017; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 23, canonical_name: "ext_0009_0017", name: "ext_0009_0017",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0009_0017, display_order: ORDER_1, params: AUTO_PARAMS_0009_0017,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0017 (ext_0009_0017) for menu/tab continuation state; pops the continuation point pushed by script dispatchers.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0017" }, Evidence { kind: EvidenceKind::Disassembly, reference: "docs/dis.txt 0003A338 pushes 2137 immediately before ext_0009_0017; similar menu dispatch sites use the same one-arg pattern" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0018 ext_0009_0018; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 24, canonical_name: "ext_0009_0018", name: "ext_0009_0018",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0009_0018, display_order: ORDER_1, params: AUTO_PARAMS_0009_0018,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0018 (ext_0009_0018) for menu/tab transition mode; pops the mode value pushed by script dispatchers.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0018" }, Evidence { kind: EvidenceKind::Disassembly, reference: "docs/dis.txt 0003A324 pushes 1 immediately before ext_0009_0018; similar menu dispatch sites use the same one-arg pattern" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0019 ext_0009_0019; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 25, canonical_name: "ext_0009_0019", name: "ext_0009_0019",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0009_0019, display_order: ORDER_1, params: AUTO_PARAMS_0009_0019,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0019 (ext_0009_0019) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0019" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0019" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_001A ext_0009_001A; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 26, canonical_name: "ext_0009_001A", name: "ext_0009_001A",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_001A, display_order: ORDER_0, params: AUTO_PARAMS_0009_001A,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:001A (ext_0009_001A) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_001A" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:001A" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_001B set_font_size; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 27, canonical_name: "set_font_size", name: "set_font_size",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_001B, display_order: ORDER_0, params: AUTO_PARAMS_0009_001B,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:001B (set_font_size) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable set_font_size" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:001B" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_001C get_font_size; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 28, canonical_name: "get_font_size", name: "get_font_size",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_001C, display_order: ORDER_0, params: AUTO_PARAMS_0009_001C,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:001C (get_font_size) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable get_font_size" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:001C" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_001D get_font_type; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 29, canonical_name: "get_font_type", name: "get_font_type",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_001D, display_order: ORDER_0, params: AUTO_PARAMS_0009_001D,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:001D (get_font_type) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable get_font_type" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:001D" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_001E set_font_effect; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 30, canonical_name: "set_font_effect", name: "set_font_effect",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_001E, display_order: ORDER_0, params: AUTO_PARAMS_0009_001E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:001E (set_font_effect) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable set_font_effect" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:001E" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_001F get_font_effect; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 31, canonical_name: "get_font_effect", name: "get_font_effect",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_001F, display_order: ORDER_0, params: AUTO_PARAMS_0009_001F,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:001F (get_font_effect) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable get_font_effect" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:001F" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0023 input_clear; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 35, canonical_name: "input_clear", name: "input_clear",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0023, display_order: ORDER_0, params: AUTO_PARAMS_0009_0023,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0023 (input_clear) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable input_clear" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0023" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0024 change_window_size; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 36, canonical_name: "change_window_size", name: "change_window_size",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0024, display_order: ORDER_0, params: AUTO_PARAMS_0009_0024,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0024 (change_window_size) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable change_window_size" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0024" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0030 set_cursor_null; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 48, canonical_name: "set_cursor_null", name: "set_cursor_null",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0030, display_order: ORDER_0, params: AUTO_PARAMS_0009_0030,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0030 (set_cursor_null) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable set_cursor_null" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0030" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0031 set_hide_cursor_time; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 49, canonical_name: "set_hide_cursor_time", name: "set_hide_cursor_time",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0031, display_order: ORDER_0, params: AUTO_PARAMS_0009_0031,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0031 (set_hide_cursor_time) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable set_hide_cursor_time" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0031" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0033 scene_skip; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 51, canonical_name: "scene_skip", name: "scene_skip",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0033, display_order: ORDER_0, params: AUTO_PARAMS_0009_0033,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0033 (scene_skip) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable scene_skip" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0033" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0034 ext_0009_0034; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 52, canonical_name: "ext_0009_0034", name: "ext_0009_0034",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0034, display_order: ORDER_0, params: AUTO_PARAMS_0009_0034,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0034 (ext_0009_0034) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0034" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0034" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0009_0035 ext_0009_0035; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 9, index: 53, canonical_name: "ext_0009_0035", name: "ext_0009_0035",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0009_0035, display_order: ORDER_0, params: AUTO_PARAMS_0009_0035,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0009:0035 (ext_0009_0035) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 9 extcall handler checked for reachable ext_0009_0035" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0009:0035" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0000 save; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 0, canonical_name: "save", name: "save",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_0000, display_order: ORDER_0, params: AUTO_PARAMS_000A_0000,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0000 (save) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable save" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0000" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0001 load; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 1, canonical_name: "load", name: "load",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_000A_0001, display_order: ORDER_1, params: AUTO_PARAMS_000A_0001,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0001 (load) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable load" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0001" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0004 save_set_thumbnail_size; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 4, canonical_name: "save_set_thumbnail_size", name: "save_set_thumbnail_size",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_0004, display_order: ORDER_0, params: AUTO_PARAMS_000A_0004,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0004 (save_set_thumbnail_size) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable save_set_thumbnail_size" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0004" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0005 thumbnail_set; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 5, canonical_name: "thumbnail_set", name: "thumbnail_set",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_0005, display_order: ORDER_0, params: AUTO_PARAMS_000A_0005,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0005 (thumbnail_set) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable thumbnail_set" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0005" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0007 save_set_font_size; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 7, canonical_name: "save_set_font_size", name: "save_set_font_size",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_0007, display_order: ORDER_0, params: AUTO_PARAMS_000A_0007,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0007 (save_set_font_size) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable save_set_font_size" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0007" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0009 is_save; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 9, canonical_name: "is_save", name: "is_save",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_000A_0009, display_order: ORDER_1, params: AUTO_PARAMS_000A_0009,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0009 (is_save) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable is_save" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0009" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_000B savepoint; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 11, canonical_name: "savepoint", name: "savepoint",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_000B, display_order: ORDER_0, params: AUTO_PARAMS_000A_000B,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:000B (savepoint) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable savepoint" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:000B" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_000C save_thumbnail_mosaic_set; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 12, canonical_name: "save_thumbnail_mosaic_set", name: "save_thumbnail_mosaic_set",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_000A_000C, display_order: ORDER_1, params: AUTO_PARAMS_000A_000C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:000C (save_thumbnail_mosaic_set) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable save_thumbnail_mosaic_set" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:000C" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_000D savetimedraw; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 13, canonical_name: "savetimedraw", name: "savetimedraw",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_000D, display_order: ORDER_0, params: AUTO_PARAMS_000A_000D,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:000D (savetimedraw) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable savetimedraw" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:000D" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_000E savedaydraw; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 14, canonical_name: "savedaydraw", name: "savedaydraw",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_000E, display_order: ORDER_0, params: AUTO_PARAMS_000A_000E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:000E (savedaydraw) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable savedaydraw" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:000E" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_000F save_set_text_rect; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 15, canonical_name: "save_set_text_rect", name: "save_set_text_rect",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_000F, display_order: ORDER_0, params: AUTO_PARAMS_000A_000F,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:000F (save_set_text_rect) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable save_set_text_rect" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:000F" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0010 savetextdraw; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 16, canonical_name: "savetextdraw", name: "savetextdraw",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_0010, display_order: ORDER_0, params: AUTO_PARAMS_000A_0010,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0010 (savetextdraw) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable savetextdraw" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0010" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0011 get_new_savefile; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 17, canonical_name: "get_new_savefile", name: "get_new_savefile",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_000A_0011, display_order: ORDER_1, params: AUTO_PARAMS_000A_0011,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0011 (get_new_savefile) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable get_new_savefile" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0011" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0016 thumbnail_renew; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 22, canonical_name: "thumbnail_renew", name: "thumbnail_renew",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_0016, display_order: ORDER_0, params: AUTO_PARAMS_000A_0016,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0016 (thumbnail_renew) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable thumbnail_renew" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0016" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0018 set_load_after_process; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 24, canonical_name: "set_load_after_process", name: "set_load_after_process",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_0018, display_order: ORDER_0, params: AUTO_PARAMS_000A_0018,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0018 (set_load_after_process) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable set_load_after_process" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0018" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0019 savesystemdata; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 25, canonical_name: "savesystemdata", name: "savesystemdata",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_0019, display_order: ORDER_0, params: AUTO_PARAMS_000A_0019,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0019 (savesystemdata) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable savesystemdata" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0019" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_001A save_set_font_effect; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 26, canonical_name: "save_set_font_effect", name: "save_set_font_effect",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_000A_001A, display_order: ORDER_1, params: AUTO_PARAMS_000A_001A,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:001A (save_set_font_effect) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable save_set_font_effect" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:001A" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_001B save_set_font_color_0x_0x; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 27, canonical_name: "save_set_font_color_0x_0x", name: "save_set_font_color_0x_0x",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_001B, display_order: ORDER_0, params: AUTO_PARAMS_000A_001B,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:001B (save_set_font_color_0x_0x) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable save_set_font_color_0x_0x" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:001B" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_001C delete_file; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 28, canonical_name: "delete_file", name: "delete_file",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_001C, display_order: ORDER_0, params: AUTO_PARAMS_000A_001C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:001C (delete_file) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable delete_file" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:001C" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_001D save_tmp_dat; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 29, canonical_name: "save_tmp_dat", name: "save_tmp_dat",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_001D, display_order: ORDER_0, params: AUTO_PARAMS_000A_001D,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:001D (save_tmp_dat) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable save_tmp_dat" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:001D" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_001E copy_file; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 30, canonical_name: "copy_file", name: "copy_file",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_001E, display_order: ORDER_0, params: AUTO_PARAMS_000A_001E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:001E (copy_file) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable copy_file" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:001E" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0020 save_lock_not_open_savefileno; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 32, canonical_name: "save_lock_not_open_savefileno", name: "save_lock_not_open_savefileno",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_0020, display_order: ORDER_0, params: AUTO_PARAMS_000A_0020,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0020 (save_lock_not_open_savefileno) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable save_lock_not_open_savefileno" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0020" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0021 is_save_lock; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 33, canonical_name: "is_save_lock", name: "is_save_lock",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_000A_0021, display_order: ORDER_1, params: AUTO_PARAMS_000A_0021,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0021 (is_save_lock) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable is_save_lock" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0021" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0022 is_prev_data; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 34, canonical_name: "is_prev_data", name: "is_prev_data",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_0022, display_order: ORDER_0, params: AUTO_PARAMS_000A_0022,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0022 (is_prev_data) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable is_prev_data" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0022" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0023 save_point_clear; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 35, canonical_name: "save_point_clear", name: "save_point_clear",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000A_0023, display_order: ORDER_0, params: AUTO_PARAMS_000A_0023,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0023 (save_point_clear) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable save_point_clear" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0023" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000A_0024 save_point_lock; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 10, index: 36, canonical_name: "save_point_lock", name: "save_point_lock",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_000A_0024, display_order: ORDER_1, params: AUTO_PARAMS_000A_0024,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSaveState],
        purpose: "Reachable Game.exe extcall 000A:0024 (save_point_lock) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 10 extcall handler checked for reachable save_point_lock" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000A:0024" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000C_0000 system_btn_set; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 12, index: 0, canonical_name: "system_btn_set", name: "system_btn_set",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_000C_0000, display_order: ORDER_2, params: AUTO_PARAMS_000C_0000,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 000C:0000 (system_btn_set) for system-button state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 12 extcall handler checked for reachable system_btn_set" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000C:0000" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000C_0001 system_btn_release; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 12, index: 1, canonical_name: "system_btn_release", name: "system_btn_release",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000C_0001, display_order: ORDER_0, params: AUTO_PARAMS_000C_0001,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 000C:0001 (system_btn_release) for system-button state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 12 extcall handler checked for reachable system_btn_release" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000C:0001" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000C_0002 system_btn_enable; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 12, index: 2, canonical_name: "system_btn_enable", name: "system_btn_enable",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_000C_0002, display_order: ORDER_1, params: AUTO_PARAMS_000C_0002,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesSelectState],
        purpose: "Reachable Game.exe extcall 000C:0002 (system_btn_enable) for system-button state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 12 extcall handler checked for reachable system_btn_enable" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000C:0002" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0004 set_voice_info; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 4, canonical_name: "set_voice_info", name: "set_voice_info",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_0004, display_order: ORDER_0, params: AUTO_PARAMS_000D_0004,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0004 (set_voice_info) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable set_voice_info" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0004" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable set_voice_info; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0005 voice_enable; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 5, canonical_name: "voice_enable", name: "voice_enable",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_0005, display_order: ORDER_0, params: AUTO_PARAMS_000D_0005,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0005 (voice_enable) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable voice_enable" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0005" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable voice_enable; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0006 is_voice_enable; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 6, canonical_name: "is_voice_enable", name: "is_voice_enable",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_0006, display_order: ORDER_0, params: AUTO_PARAMS_000D_0006,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0006 (is_voice_enable) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable is_voice_enable" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0006" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable is_voice_enable; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0007 ext_000D_0007; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 7, canonical_name: "ext_000D_0007", name: "ext_000D_0007",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_0007, display_order: ORDER_0, params: AUTO_PARAMS_000D_0007,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0007 (ext_000D_0007) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable ext_000D_0007" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0007" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_000D_0007; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0008 bgv_play; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 8, canonical_name: "bgv_play", name: "bgv_play",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_0008, display_order: ORDER_0, params: AUTO_PARAMS_000D_0008,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0008 (bgv_play) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable bgv_play" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0008" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable bgv_play; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_000B get_voice_ex_volume; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 11, canonical_name: "get_voice_ex_volume", name: "get_voice_ex_volume",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_000B, display_order: ORDER_0, params: AUTO_PARAMS_000D_000B,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:000B (get_voice_ex_volume) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable get_voice_ex_volume" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:000B" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable get_voice_ex_volume; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_000C set_voice_ex_volume; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 12, canonical_name: "set_voice_ex_volume", name: "set_voice_ex_volume",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_000C, display_order: ORDER_0, params: AUTO_PARAMS_000D_000C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:000C (set_voice_ex_volume) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable set_voice_ex_volume" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:000C" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable set_voice_ex_volume; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_000E voice_autopan_initialize; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 14, canonical_name: "voice_autopan_initialize", name: "voice_autopan_initialize",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_000E, display_order: ORDER_0, params: AUTO_PARAMS_000D_000E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:000E (voice_autopan_initialize) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable voice_autopan_initialize" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:000E" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable voice_autopan_initialize; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_000F voice_autopan_enable; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 15, canonical_name: "voice_autopan_enable", name: "voice_autopan_enable",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_000F, display_order: ORDER_0, params: AUTO_PARAMS_000D_000F,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:000F (voice_autopan_enable) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable voice_autopan_enable" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:000F" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable voice_autopan_enable; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0010 set_voice_autopan_size_over; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 16, canonical_name: "set_voice_autopan_size_over", name: "set_voice_autopan_size_over",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_000D_0010, display_order: ORDER_2, params: AUTO_PARAMS_000D_0010,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0010 (set_voice_autopan_size_over) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable set_voice_autopan_size_over" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0010" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable set_voice_autopan_size_over; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0011 is_voice_autopan_enable; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 17, canonical_name: "is_voice_autopan_enable", name: "is_voice_autopan_enable",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_0011, display_order: ORDER_0, params: AUTO_PARAMS_000D_0011,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0011 (is_voice_autopan_enable) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable is_voice_autopan_enable" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0011" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable is_voice_autopan_enable; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0013 bgv_pause; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 19, canonical_name: "bgv_pause", name: "bgv_pause",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_0013, display_order: ORDER_0, params: AUTO_PARAMS_000D_0013,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0013 (bgv_pause) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable bgv_pause" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0013" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable bgv_pause; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0014 bgv_mute; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 20, canonical_name: "bgv_mute", name: "bgv_mute",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_0014, display_order: ORDER_0, params: AUTO_PARAMS_000D_0014,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0014 (bgv_mute) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable bgv_mute" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0014" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable bgv_mute; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0015 set_bgv_volume; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 21, canonical_name: "set_bgv_volume", name: "set_bgv_volume",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_0015, display_order: ORDER_0, params: AUTO_PARAMS_000D_0015,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0015 (set_bgv_volume) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable set_bgv_volume" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0015" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable set_bgv_volume; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0016 get_bgv_volume; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 22, canonical_name: "get_bgv_volume", name: "get_bgv_volume",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_0016, display_order: ORDER_0, params: AUTO_PARAMS_000D_0016,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0016 (get_bgv_volume) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable get_bgv_volume" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0016" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable get_bgv_volume; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000D_0018 voice_mute; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 13, index: 24, canonical_name: "voice_mute", name: "voice_mute",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000D_0018, display_order: ORDER_0, params: AUTO_PARAMS_000D_0018,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesSound],
        purpose: "Reachable Game.exe extcall 000D:0018 (voice_mute) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 13 extcall handler checked for reachable voice_mute" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000D:0018" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable voice_mute; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_0000 history_init_0x_0x; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 0, canonical_name: "history_init_0x_0x", name: "history_init_0x_0x",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_0000, display_order: ORDER_0, params: AUTO_PARAMS_000E_0000,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:0000 (history_init_0x_0x) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable history_init_0x_0x" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:0000" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_0001 historybegin_lpbyte_ptagdata_sztext; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 1, canonical_name: "historybegin_lpbyte_ptagdata_sztext", name: "historybegin_lpbyte_ptagdata_sztext",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_0001, display_order: ORDER_0, params: AUTO_PARAMS_000E_0001,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:0001 (historybegin_lpbyte_ptagdata_sztext) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable historybegin_lpbyte_ptagdata_sztext" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:0001" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_0002 history_end; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 2, canonical_name: "history_end", name: "history_end",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_0002, display_order: ORDER_0, params: AUTO_PARAMS_000E_0002,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:0002 (history_end) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable history_end" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:0002" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_0003 ext_000E_0003; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 3, canonical_name: "ext_000E_0003", name: "ext_000E_0003",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_0003, display_order: ORDER_0, params: AUTO_PARAMS_000E_0003,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:0003 (ext_000E_0003) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable ext_000E_0003" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:0003" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_0004 ext_000E_0004; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 4, canonical_name: "ext_000E_0004", name: "ext_000E_0004",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_0004, display_order: ORDER_0, params: AUTO_PARAMS_000E_0004,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:0004 (ext_000E_0004) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable ext_000E_0004" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:0004" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_0005 history_get_height; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 5, canonical_name: "history_get_height", name: "history_get_height",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_0005, display_order: ORDER_0, params: AUTO_PARAMS_000E_0005,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:0005 (history_get_height) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable history_get_height" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:0005" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_0006 ext_000E_0006; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 6, canonical_name: "ext_000E_0006", name: "ext_000E_0006",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_0006, display_order: ORDER_0, params: AUTO_PARAMS_000E_0006,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:0006 (ext_000E_0006) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable ext_000E_0006" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:0006" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_0007 ext_000E_0007; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 7, canonical_name: "ext_000E_0007", name: "ext_000E_0007",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_0007, display_order: ORDER_0, params: AUTO_PARAMS_000E_0007,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:0007 (ext_000E_0007) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable ext_000E_0007" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:0007" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_0008 ext_000E_0008; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 8, canonical_name: "ext_000E_0008", name: "ext_000E_0008",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_0008, display_order: ORDER_0, params: AUTO_PARAMS_000E_0008,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:0008 (ext_000E_0008) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable ext_000E_0008" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:0008" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_0009 ext_000E_0009; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 9, canonical_name: "ext_000E_0009", name: "ext_000E_0009",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_0009, display_order: ORDER_0, params: AUTO_PARAMS_000E_0009,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:0009 (ext_000E_0009) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable ext_000E_0009" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:0009" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_000A history_set_rect; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 10, canonical_name: "history_set_rect", name: "history_set_rect",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_000A, display_order: ORDER_0, params: AUTO_PARAMS_000E_000A,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:000A (history_set_rect) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable history_set_rect" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:000A" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_000B history_clear; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 11, canonical_name: "history_clear", name: "history_clear",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_000B, display_order: ORDER_0, params: AUTO_PARAMS_000E_000B,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:000B (history_clear) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable history_clear" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:000B" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000E_000C history_set; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 14, index: 12, canonical_name: "history_set", name: "history_set",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000E_000C, display_order: ORDER_0, params: AUTO_PARAMS_000E_000C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ChangesHistoryState],
        purpose: "Reachable Game.exe extcall 000E:000C (history_set) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 14 extcall handler checked for reachable history_set" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000E:000C" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000F_0002 set_window_text; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 15, index: 2, canonical_name: "set_window_text", name: "set_window_text",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_000F_0002, display_order: ORDER_0, params: AUTO_PARAMS_000F_0002,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 000F:0002 (set_window_text) for misc system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 15 extcall handler checked for reachable set_window_text" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000F:0002" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000F_0004 ext_000F_0004; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 15, index: 4, canonical_name: "ext_000F_0004", name: "ext_000F_0004",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 3, pop_order: AUTO_POP_000F_0004, display_order: ORDER_3, params: AUTO_PARAMS_000F_0004,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 000F:0004 (ext_000F_0004) for misc system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 15 extcall handler checked for reachable ext_000F_0004" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000F:0004" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 000F_0005 ext_000F_0005; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 15, index: 5, canonical_name: "ext_000F_0005", name: "ext_000F_0005",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_000F_0005, display_order: ORDER_1, params: AUTO_PARAMS_000F_0005,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 000F:0005 (ext_000F_0005) for misc system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 15 extcall handler checked for reachable ext_000F_0005" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 000F:0005" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0010_0000 effect_stop_skip; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 16, index: 0, canonical_name: "effect_stop_skip", name: "effect_stop_skip",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0010_0000, display_order: ORDER_0, params: AUTO_PARAMS_0010_0000,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0010:0000 (effect_stop_skip) for window/effect state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 16 extcall handler checked for reachable effect_stop_skip" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0010:0000" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable effect_stop_skip; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0010_0001 ext_0010_0001; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 16, index: 1, canonical_name: "ext_0010_0001", name: "ext_0010_0001",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0010_0001, display_order: ORDER_0, params: AUTO_PARAMS_0010_0001,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0010:0001 (ext_0010_0001) for window/effect state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 16 extcall handler checked for reachable ext_0010_0001" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0010:0001" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0010_0001; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0010_0002 ext_0010_0002; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 16, index: 2, canonical_name: "ext_0010_0002", name: "ext_0010_0002",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0010_0002, display_order: ORDER_2, params: AUTO_PARAMS_0010_0002,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0010:0002 (ext_0010_0002) for window/effect state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 16 extcall handler checked for reachable ext_0010_0002" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0010:0002" }, Evidence { kind: EvidenceKind::PalSqlite, reference: "reverse/PAL.sqlite exports checked by subsystem for reachable ext_0010_0002; exact export recorded when known" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0000 action_run_count_over; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 0, canonical_name: "action_run_count_over", name: "action_run_count_over",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0011_0000, display_order: ORDER_0, params: AUTO_PARAMS_0011_0000,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0000 (action_run_count_over) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable action_run_count_over" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0000" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0001 action_sync_run_count_over; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 1, canonical_name: "action_sync_run_count_over", name: "action_sync_run_count_over",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0011_0001, display_order: ORDER_1, params: AUTO_PARAMS_0011_0001,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0001 (action_sync_run_count_over) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable action_sync_run_count_over" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0001" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0003 action_clear_count_over; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 3, canonical_name: "action_clear_count_over", name: "action_clear_count_over",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0011_0003, display_order: ORDER_1, params: AUTO_PARAMS_0011_0003,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0003 (action_clear_count_over) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable action_clear_count_over" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0003" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0005 ext_0011_0005; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 5, canonical_name: "ext_0011_0005", name: "ext_0011_0005",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 6, pop_order: AUTO_POP_0011_0005, display_order: ORDER_6, params: AUTO_PARAMS_0011_0005,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0005 (ext_0011_0005) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_0005" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0005" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0006 ext_0011_0006; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 6, canonical_name: "ext_0011_0006", name: "ext_0011_0006",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0011_0006, display_order: ORDER_2, params: AUTO_PARAMS_0011_0006,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0006 (ext_0011_0006) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_0006" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0006" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0007 ext_0011_0007; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 7, canonical_name: "ext_0011_0007", name: "ext_0011_0007",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 3, pop_order: AUTO_POP_0011_0007, display_order: ORDER_3, params: AUTO_PARAMS_0011_0007,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0007 (ext_0011_0007) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_0007" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0007" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0008 ext_0011_0008; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 8, canonical_name: "ext_0011_0008", name: "ext_0011_0008",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 4, pop_order: AUTO_POP_0011_0008, display_order: ORDER_4, params: AUTO_PARAMS_0011_0008,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0008 (ext_0011_0008) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_0008" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0008" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0009 ext_0011_0009; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 9, canonical_name: "ext_0011_0009", name: "ext_0011_0009",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0011_0009, display_order: ORDER_0, params: AUTO_PARAMS_0011_0009,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0009 (ext_0011_0009) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_0009" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0009" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_000A ext_0011_000A; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 10, canonical_name: "ext_0011_000A", name: "ext_0011_000A",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0011_000A, display_order: ORDER_0, params: AUTO_PARAMS_0011_000A,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:000A (ext_0011_000A) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_000A" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:000A" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_000B ext_0011_000B; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 11, canonical_name: "ext_0011_000B", name: "ext_0011_000B",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0011_000B, display_order: ORDER_1, params: AUTO_PARAMS_0011_000B,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:000B (ext_0011_000B) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_000B" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:000B" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_000E ext_0011_000E; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 14, canonical_name: "ext_0011_000E", name: "ext_0011_000E",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 4, pop_order: AUTO_POP_0011_000E, display_order: ORDER_4, params: AUTO_PARAMS_0011_000E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:000E (ext_0011_000E) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_000E" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:000E" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_000F ext_0011_000F; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 15, canonical_name: "ext_0011_000F", name: "ext_0011_000F",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0011_000F, display_order: ORDER_0, params: AUTO_PARAMS_0011_000F,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:000F (ext_0011_000F) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_000F" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:000F" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0011 ext_0011_0011; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 17, canonical_name: "ext_0011_0011", name: "ext_0011_0011",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0011_0011, display_order: ORDER_1, params: AUTO_PARAMS_0011_0011,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0011 (ext_0011_0011) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_0011" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0011" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0014 ext_0011_0014; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 20, canonical_name: "ext_0011_0014", name: "ext_0011_0014",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 3, pop_order: AUTO_POP_0011_0014, display_order: ORDER_3, params: AUTO_PARAMS_0011_0014,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0014 (ext_0011_0014) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_0014" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0014" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0015 ext_0011_0015; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 21, canonical_name: "ext_0011_0015", name: "ext_0011_0015",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0011_0015, display_order: ORDER_0, params: AUTO_PARAMS_0011_0015,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0015 (ext_0011_0015) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_0015" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0015" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0017 set_active_action; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 23, canonical_name: "set_active_action", name: "set_active_action",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0011_0017, display_order: ORDER_1, params: AUTO_PARAMS_0011_0017,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0017 (set_active_action) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable set_active_action" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0017" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_0018 get_active_action; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 24, canonical_name: "get_active_action", name: "get_active_action",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0011_0018, display_order: ORDER_1, params: AUTO_PARAMS_0011_0018,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:0018 (get_active_action) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable get_active_action" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:0018" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_001C action_uninit; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 28, canonical_name: "action_uninit", name: "action_uninit",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0011_001C, display_order: ORDER_0, params: AUTO_PARAMS_0011_001C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:001C (action_uninit) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable action_uninit" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:001C" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_001D ext_0011_001D; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 29, canonical_name: "ext_0011_001D", name: "ext_0011_001D",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 4, pop_order: AUTO_POP_0011_001D, display_order: ORDER_4, params: AUTO_PARAMS_0011_001D,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:001D (ext_0011_001D) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable ext_0011_001D" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:001D" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0011_001E set_action_clear; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 17, index: 30, canonical_name: "set_action_clear", name: "set_action_clear",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0011_001E, display_order: ORDER_0, params: AUTO_PARAMS_0011_001E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0011:001E (set_action_clear) for action/tween scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 17 extcall handler checked for reachable set_action_clear" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0011:001E" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0001 app_exec; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 1, canonical_name: "app_exec", name: "app_exec",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0001, display_order: ORDER_0, params: AUTO_PARAMS_0012_0001,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0001 (app_exec) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable app_exec" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0001" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0003 ext_0012_0003; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 3, canonical_name: "ext_0012_0003", name: "ext_0012_0003",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0003, display_order: ORDER_0, params: AUTO_PARAMS_0012_0003,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0003 (ext_0012_0003) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_0003" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0003" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0004 ext_0012_0004; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 4, canonical_name: "ext_0012_0004", name: "ext_0012_0004",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0004, display_order: ORDER_0, params: AUTO_PARAMS_0012_0004,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0004 (ext_0012_0004) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_0004" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0004" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0005 ext_0012_0005; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 5, canonical_name: "ext_0012_0005", name: "ext_0012_0005",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0005, display_order: ORDER_0, params: AUTO_PARAMS_0012_0005,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0005 (ext_0012_0005) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_0005" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0005" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0006 ext_0012_0006; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 6, canonical_name: "ext_0012_0006", name: "ext_0012_0006",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 2, pop_order: AUTO_POP_0012_0006, display_order: ORDER_2, params: AUTO_PARAMS_0012_0006,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0006 (ext_0012_0006) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_0006" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0006" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0007 ext_0012_0007; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 7, canonical_name: "ext_0012_0007", name: "ext_0012_0007",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0007, display_order: ORDER_0, params: AUTO_PARAMS_0012_0007,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0007 (ext_0012_0007) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_0007" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0007" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0008 file_exist; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 8, canonical_name: "file_exist", name: "file_exist",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0008, display_order: ORDER_0, params: AUTO_PARAMS_0012_0008,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0008 (file_exist) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable file_exist" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0008" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0009 wsprint; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 9, canonical_name: "wsprint", name: "wsprint",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0009, display_order: ORDER_0, params: AUTO_PARAMS_0012_0009,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0009 (wsprint) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable wsprint" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0009" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_000A check_disc; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 10, canonical_name: "check_disc", name: "check_disc",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_000A, display_order: ORDER_0, params: AUTO_PARAMS_0012_000A,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:000A (check_disc) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable check_disc" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:000A" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_000C ext_0012_000C; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 12, canonical_name: "ext_0012_000C", name: "ext_0012_000C",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0012_000C, display_order: ORDER_1, params: AUTO_PARAMS_0012_000C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:000C (ext_0012_000C) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_000C" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:000C" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_000D ext_0012_000D; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 13, canonical_name: "ext_0012_000D", name: "ext_0012_000D",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0012_000D, display_order: ORDER_1, params: AUTO_PARAMS_0012_000D,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:000D (ext_0012_000D) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_000D" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:000D" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_000E update_access; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 14, canonical_name: "update_access", name: "update_access",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_000E, display_order: ORDER_0, params: AUTO_PARAMS_0012_000E,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:000E (update_access) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable update_access" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:000E" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_000F ext_0012_000F; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 15, canonical_name: "ext_0012_000F", name: "ext_0012_000F",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_000F, display_order: ORDER_0, params: AUTO_PARAMS_0012_000F,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:000F (ext_0012_000F) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_000F" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:000F" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0011 ext_0012_0011; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 17, canonical_name: "ext_0012_0011", name: "ext_0012_0011",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0012_0011, display_order: ORDER_1, params: AUTO_PARAMS_0012_0011,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0011 (ext_0012_0011) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_0011" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0011" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0012 ext_0012_0012; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 18, canonical_name: "ext_0012_0012", name: "ext_0012_0012",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0012, display_order: ORDER_0, params: AUTO_PARAMS_0012_0012,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0012 (ext_0012_0012) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_0012" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0012" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0015 ext_0012_0015; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 21, canonical_name: "ext_0012_0015", name: "ext_0012_0015",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0015, display_order: ORDER_0, params: AUTO_PARAMS_0012_0015,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0015 (ext_0012_0015) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_0015" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0015" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_001C ext_0012_001C; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 28, canonical_name: "ext_0012_001C", name: "ext_0012_001C",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 1, pop_order: AUTO_POP_0012_001C, display_order: ORDER_1, params: AUTO_PARAMS_0012_001C,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:001C (ext_0012_001C) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_001C" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:001C" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_001D ext_0012_001D; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 29, canonical_name: "ext_0012_001D", name: "ext_0012_001D",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_001D, display_order: ORDER_0, params: AUTO_PARAMS_0012_001D,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:001D (ext_0012_001D) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_001D" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:001D" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0020 close_file_not_handle; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 32, canonical_name: "close_file_not_handle", name: "close_file_not_handle",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0020, display_order: ORDER_0, params: AUTO_PARAMS_0012_0020,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0020 (close_file_not_handle) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable close_file_not_handle" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0020" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0022 file_string; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 34, canonical_name: "file_string", name: "file_string",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0022, display_order: ORDER_0, params: AUTO_PARAMS_0012_0022,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0022 (file_string) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable file_string" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0022" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0023 set_last_process; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 35, canonical_name: "set_last_process", name: "set_last_process",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0023, display_order: ORDER_0, params: AUTO_PARAMS_0012_0023,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0023 (set_last_process) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable set_last_process" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0023" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0012_0028 ext_0012_0028; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 18, index: 40, canonical_name: "ext_0012_0028", name: "ext_0012_0028",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0012_0028, display_order: ORDER_0, params: AUTO_PARAMS_0012_0028,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::ReadsFile],
        purpose: "Reachable Game.exe extcall 0012:0028 (ext_0012_0028) for file/INI/string helper state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 18 extcall handler checked for reachable ext_0012_0028" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0012:0028" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0014_0000 random; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 20, index: 0, canonical_name: "random", name: "random",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0014_0000, display_order: ORDER_0, params: AUTO_PARAMS_0014_0000,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0014:0000 (random) for random number state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 20 extcall handler checked for reachable random" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0014:0000" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0015_0000 create_thread; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 21, index: 0, canonical_name: "create_thread", name: "create_thread",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0015_0000, display_order: ORDER_0, params: AUTO_PARAMS_0015_0000,
        return_kind: ReturnKind::Handle, returns: true,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0015:0000 (create_thread) for thread state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 21 extcall handler checked for reachable create_thread" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0015:0000" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0015_0002 ext_0015_0002; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 21, index: 2, canonical_name: "ext_0015_0002", name: "ext_0015_0002",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0015_0002, display_order: ORDER_0, params: AUTO_PARAMS_0015_0002,
        return_kind: ReturnKind::Void, returns: false,
        side_effects: &[SideEffect::CreatesTask],
        purpose: "Reachable Game.exe extcall 0015:0002 (ext_0015_0002) for thread state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 21 extcall handler checked for reachable ext_0015_0002" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0015:0002" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0017_0000 create_message; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 23, index: 0, canonical_name: "create_message", name: "create_message",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0017_0000, display_order: ORDER_0, params: AUTO_PARAMS_0017_0000,
        return_kind: ReturnKind::Handle, returns: true,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0017:0000 (create_message) for message queue state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 23 extcall handler checked for reachable create_message" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0017:0000" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0017_0001 get_message; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 23, index: 1, canonical_name: "get_message", name: "get_message",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0017_0001, display_order: ORDER_0, params: AUTO_PARAMS_0017_0001,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0017:0001 (get_message) for message queue state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 23 extcall handler checked for reachable get_message" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0017:0001" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
    // Game.sqlite/PAL.sqlite evidence checked for reachable extcall 0017_0002 get_message_param; promote to extsig.rs when handler EA/export is fully named.
    ExtSig {
        category: 23, index: 2, canonical_name: "get_message_param", name: "get_message_param",
        game_handler_ea: None, pal_export_name: None, pal_export_ea: None,
        pop_count: 0, pop_order: AUTO_POP_0017_0002, display_order: ORDER_0, params: AUTO_PARAMS_0017_0002,
        return_kind: ReturnKind::Integer, returns: true,
        side_effects: &[SideEffect::UnknownSideEffect],
        purpose: "Reachable Game.exe extcall 0017:0002 (get_message_param) for message queue state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args.",
        evidence: &[Evidence { kind: EvidenceKind::GameSqlite, reference: "reverse/Game.sqlite category 23 extcall handler checked for reachable get_message_param" }, Evidence { kind: EvidenceKind::Disassembly, reference: "out/extcall_report.json and docs/dis.txt reachable extcall 0017:0002" }],
        implementation_status: ImplStatus::Blocked, decompiler_status: ImplStatus::Blocked,
    },
];

pub fn lookup_auto_sig(category: u16, index: u16) -> Option<&'static ExtSig> {
    AUTO_REACHABLE_SIGNATURES
        .iter()
        .find(|sig| sig.category == category && sig.index == index)
}

pub fn auto_signatures() -> &'static [ExtSig] {
    AUTO_REACHABLE_SIGNATURES
}
