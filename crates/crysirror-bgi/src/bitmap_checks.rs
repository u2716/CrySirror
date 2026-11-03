use super::*;
impl BgiVm {
    pub(super) fn graph91_check_bitmap_operation(
        &mut self,
        id: u16,
        a: &[i32],
    ) -> crysirror_bgi_vm::VmResult<()> {
        let fail = |text: String| Err(crysirror_bgi_vm::VmError::Runtime(text));
        let exists = |bitmap: i32| self.bitmaps.contains_key(&(bitmap as u32));
        match id {
            // sub_402F30 / sub_402FD0 -> sub_416B60 / sub_4179B0.
            0x18 | 0x19 => {
                target_checks::effect_level(a[9])?;
                if !exists(a[0]) {
                    return fail(format!(
                        "出力先として指定されたビットマップ [ {} ] は無効です",
                        a[0]
                    ));
                }
                if !exists(a[3]) {
                    return fail(format!(
                        "参照元として指定されたビットマップ [ {} ] は無効です",
                        a[3]
                    ));
                }
                if a[7] == 0 || a[8] == 0 {
                    return fail(format!(
                        "無効な伸縮倍率 [ {} , {} ] が指定されました",
                        a[7], a[8]
                    ));
                }
            }
            // sub_4034F0.
            0x1a => {
                if !exists(a[0]) {
                    return fail(format!(
                        "指定された出力先ビットマップ [ {} ] は無効です",
                        a[0]
                    ));
                }
                if !exists(a[1]) {
                    return fail(format!(
                        "指定された参照元ビットマップ [ {} ] は無効です",
                        a[1]
                    ));
                }
            }
            // sub_403070.
            0x1b => {
                if !exists(a[0]) {
                    return fail(format!(
                        "指定された出力先ビットマップ [ {} ] は無効です",
                        a[0]
                    ));
                }
                if !exists(a[1]) {
                    return fail(format!(
                        "指定された参照元ビットマップ [ {} ] は無効です",
                        a[1]
                    ));
                }
                if a[3] as u32 > 0x10000 || a[2] as u32 > 0x10000 {
                    return fail(format!(
                        "指定された集合度 [ {} ／ {} ] は無効です",
                        a[2], a[3]
                    ));
                }
                if a[4] as u32 > 0x100 {
                    return fail(format!("指定された輝度減衰量 [ {} ] は無効です", a[4]));
                }
            }
            // sub_402B90: the destination is recreated at the scaled size
            // with the source format.
            0x1c => {
                let Some(info) = self.bitmap_info(a[1]) else {
                    return fail(format!(
                        "指定された参照元ビットマップ [ {} ] は無効です",
                        a[1]
                    ));
                };
                if !matches!(info.format, 1 | 2) {
                    return fail(format!(
                        "指定された参照元ビットマップ [ {} ] はTRUECOLORではありません",
                        a[1]
                    ));
                }
                let width = (a[2] as u32).wrapping_mul(info.width) >> 16;
                let height = (a[3] as u32).wrapping_mul(info.height) >> 16;
                if width == 0 || height == 0 {
                    return fail(format!(
                        "無効なストレッチレート [ {} , {} ] は無効です",
                        a[2], a[3]
                    ));
                }
                if !(0..0x4000).contains(&a[0]) {
                    return fail(format!(
                        "指定された格納先ビットマップ [ {} ] は無効です",
                        a[0]
                    ));
                }
            }
            // sub_403110 -> sub_419CF0: types 0..=5.
            0x1d => {
                target_checks::effect_level(a[4])?;
                if !exists(a[0]) {
                    return fail(format!(
                        "指定された出力先ビットマップ [ {} ] は無効です",
                        a[0]
                    ));
                }
                if !exists(a[1]) {
                    return fail(format!(
                        "指定された参照元ビットマップ [ {} ] は無効です",
                        a[1]
                    ));
                }
                if a[2] as u32 > 5 {
                    return fail(format!("指定された処理タイプ [ {} ] は無効です", a[2]));
                }
            }
            // sub_403190: the format is only checked when the source,
            // moved by (-p2, -p3), overlaps the target.
            0x1e => {
                if !exists(a[0]) {
                    return fail(format!(
                        "指定された対象ビットマップ [ {} ] は無効です",
                        a[0]
                    ));
                }
                if !exists(a[1]) {
                    return fail(format!(
                        "指定された参照元ビットマップ [ {} ] は無効です",
                        a[1]
                    ));
                }
                let target = self.bitmap_info(a[0]);
                let source = self.bitmap_info(a[1]);
                if let (Some(target), Some(source)) = (target, source) {
                    let (left, top) = (-(a[2] as i64), -(a[3] as i64));
                    let overlaps = left < target.width as i64
                        && top < target.height as i64
                        && left + source.width as i64 > 0
                        && top + source.height as i64 > 0;
                    if overlaps && !matches!(source.format, 2 | 3) {
                        return fail(format!(
                            "指定された参照元ビットマップ [ {} ] はグレイスケールではありません",
                            a[1]
                        ));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
