// SPDX-License-Identifier: MPL-2.0
//! CVNS 2004-05-15 x86 syscall routes recovered from the original dispatcher.
//! Unreviewed routes are tracked by SATD-cvns-20040515-profile.

/// Pending CVNS selectors without a reviewed CMVS counterpart are kept in this
/// separate command space, so `0x2000 + command` reads `0x2800 + selector`.
pub(crate) const NATIVE: u16 = 0x800;

/// Maps a CVNS dispatcher index to the machine's pending command.
///
/// A CMVS command is used only when the CVNS case at the recovered address has
/// the same operands, cleanup, result and observable effect as the 2016 case.
pub(super) fn route(command: u16) -> u16 {
    let opcode = 0x2000 + command;
    let cmvs = match opcode {
        // 0x42c3f0 copies the main script name, including during a slot-1 call.
        0x2013 => 0x2082,
        // GetLocalTime: YYYYMMDD, HHMMSS and weekday in registers 4..6.
        0x201c => 0x200d,
        // 0x435df0 always enables the load group and sets the save group gate.
        0x21a2 => 0x22b4,
        // The dispatcher writes a constant one to result 0.
        0x203c => 0x23b2,
        // 0x411e00 and 0x411da0 query/clear the image and main script lists.
        0x21c8 => 0x2120,
        0x21c9 => 0x2121,
        0x21cb => 0x2122,
        0x21cc => 0x2123,
        // 0x42e5e0 copies the title into the frame and calls SetWindowTextA.
        0x2178 => 0x2160,
        // 0x412830 returns the active display mode (3 for 32-bit fullscreen).
        0x217b => 0x2163,
        // 0x42e540 loads an install-relative icon and sends WM_SETICON.
        0x2168 => 0x208f,
        // 0x430d30 appends a cursor name and returns the 1-based count.
        0x21d4 => 0x2098,
        // 0x437e00 keeps the cursor for the active or an unregistered index.
        0x21d5 => 0x2099,
        // 0x42fd70 limits only the checkpoint's newest-first dialogue history.
        0x21a5 => 0x22b1,
        // 0x4258e0 copies these three fields into the CSP1 header at 528..540.
        0x21a9 => 0x22b9,
        0x21aa => 0x22ba,
        // 0x42fc50 stores the thumbnail width at SP-4 and height at SP-8.
        0x21a6 => 0x22b0,
        // 0x427110 captures the presented frame as a bottom-up BGR24 BMP.
        0x21af => 0x22b8,
        // Cases 450 and 451 of 0x432550 set the gates +20420/+20424 of the
        // image and main-script name lists that 0x411e60 appends once each.
        0x21c2 => 0x212a,
        0x21c3 => 0x212b,
        // 0x42a9f0 calls entry `index & 63` like the 2016 callback invoke.
        0x202b => 0x208a,
        // 0x42e610 appends the scene text after the 0x8140 separator, and
        // 0x42e6b0 keeps a save title below 128 bytes.
        0x2179 => 0x2161,
        0x217a => 0x2162,
        // 0x42fbf0 gates the right-click popup (+20440) like the 2016 0x22cd.
        0x2068 => 0x22cd,
        // 0x4274d0/0x427500 reset and read the ten frame-time counters (+22908).
        0x201a => 0x200b,
        0x201b => 0x200c,
        // 0x429da0, 0x42a070 and 0x42a1d0 load, start and stop effect channels
        // with the 2016 operands; the profile differences are in
        // `CmvsVm::pending_effect_sound`. 0x42a240 stops every channel.
        0x20b0..=0x20b3 => opcode,
        0x20ad => 0x20ba,
        0x20ae => 0x20b4,
        0x20b5 => 0x20b6,
        // 0x42b170..0x42b240 compare signed words. 0x42b310 retains the
        // base for exponents <= 1 and multiplies modulo 2^32 otherwise.
        0x20e0 => 0x20d0,
        0x20e1 => 0x20d1,
        0x20e2 => 0x20d2,
        0x20e3 => 0x20d3,
        0x20e6 => 0x20d6,
        // 0x429820 plays the music name at SP-4 with fade SP-8 and loop SP-12,
        // 0x408680 stops it and 0x4298b0 fades it out over SP-4 milliseconds.
        0x20a0..=0x20a2 => opcode,
        0x20b4 => 0x20b5,
        // 0x4276xx posts WM_DESTROY like the 2016 quit 0x2001.
        0x2006 => 0x2001,
        // 0x431890 opens the same two-button exit dialog with the same
        // caption, prompt, 0/1 result and no stack operands.
        0x21b0 => 0x213a,
        // 0x42fc20 (0x21a0) snapshots the VM after this instruction, as the 2016
        // scene checkpoint 0x22b2 does. The CSP1 save image is built from it.
        0x21a0 => 0x22b2,
        // 0x42b... converts SP-4/SP-8 to screen coordinates for SetCursorPos.
        0x2033 => 0x20cc,
        // 0x42b630 uses the same seven operands, winding region and current pointer.
        0x2032 => 0x20cb,
        // Show/hide the class cursor at 0x406d50.
        0x2034 => 0x20cd,
        _ => return command | NATIVE,
    };
    cmvs - 0x2000
}

/// The CVNS selector of a pending native command.
pub(crate) fn selector(opcode: u16) -> Option<u16> {
    (opcode >= 0x2000 + NATIVE).then(|| opcode - NATIVE)
}

/// The x86 jump table shares this exact no-effect, no-cleanup handler at 0x432fd5.
pub(super) fn returns_without_effect(opcode: u16) -> bool {
    matches!(opcode,
        0x2001
        | 0x200e..=0x200f
        | 0x2011
        | 0x2015..=0x2017
        | 0x201d..=0x201e
        | 0x2027
        | 0x202d..=0x202f
        | 0x2035..=0x2037
        | 0x203e..=0x203f
        | 0x2043..=0x2046
        | 0x204d..=0x204f
        | 0x2054
        | 0x2060..=0x2061
        | 0x2064..=0x2065
        | 0x206a
        | 0x206e..=0x206f
        | 0x2073
        | 0x2087
        | 0x2089..=0x208f
        | 0x2095..=0x209f
        | 0x20a3
        | 0x20a7..=0x20ac
        | 0x20bf
        | 0x20c1..=0x20c4
        | 0x20cd..=0x20cf
        | 0x20ec..=0x20ef
        | 0x20fe..=0x20ff
        | 0x2103..=0x2107
        | 0x210f
        | 0x2115..=0x2116
        | 0x211c
        | 0x2127
        | 0x2132..=0x2135
        | 0x2137
        | 0x2149..=0x214e
        | 0x2157
        | 0x215b..=0x215f
        | 0x2161..=0x2162
        | 0x2164..=0x2167
        | 0x2169..=0x2177
        | 0x217e
        | 0x2184..=0x2187
        | 0x2197..=0x219f
        | 0x21a8
        | 0x21ad
        | 0x21b3..=0x21b5
        | 0x21b8..=0x21bf
        | 0x21c5..=0x21c7
        | 0x21cd..=0x21d3
        | 0x21d6..=0x21eb
        | 0x21ee..=0x21ff
        | 0x2202..=0x221f
    )
}

/// Engine fields set by CVNS VM-owned selectors.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CvnsState {
    /// `0x210e` sets +0x4fa4; `0x433bb0` then compares one bank stripe per frame
    /// with its mirror. Every CrySirror write updates both copies.
    pub audit: bool,
    /// `0x210b`..`0x210d`: first persistent bit, integer and string cell
    /// (+19176, +19180, +19184) used by the `save000.dat` reader `0x412170`.
    pub persistent_start: [u32; 3],
    /// `0x2014` packs SP-4 into the high word and SP-8 into the low word (+20384).
    pub version: u32,
    /// `0x206b` (+20444): `0x411cd0` skips the next right-click popup.
    pub popup_suppressed: bool,
    /// Field +19208, cleared in 0x410f50 and latched by case 452 of 0x432550.
    pub external_slot: bool,
    /// Music owner +160..4256 and +4256. The Windows dispatcher in this
    /// profile selects file playback or disabled audio at start and on toggles.
    pub cd_tracks: std::collections::BTreeMap<u8, Vec<u8>>,
    pub last_cd_track: u32,
}
