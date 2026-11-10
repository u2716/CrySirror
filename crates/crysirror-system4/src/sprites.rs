// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. xsystem4 a85467d SACT2.c / sprite.c native handle policy.
//! Native sprite properties mapped to retained CrySirror layers. There is no
//! private pixel buffer, image decoder, font renderer or general compositor.
use anyhow::{Context, Result, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub enum Surface {
    Solid([u8; 4]),
    Image(String),
}
#[derive(Clone, Debug)]
pub struct Sprite {
    pub position: [i32; 2],
    pub size: [u32; 2],
    pub z: i32,
    pub opacity: u8,
    pub visible: bool,
    pub alpha: bool,
    pub brightness: u8,
    pub draw_method: i32,
    pub surface: Option<Surface>,
    pub text_home: [i32; 2],
    pub text_position: [i32; 2],
    pub text_char_space: i32,
    pub text_line_space: i32,
    pub text_line_height: i32,
}
impl Default for Sprite {
    fn default() -> Self {
        Self {
            position: [0, 0],
            size: [0, 0],
            z: 0,
            opacity: 255,
            //Native SactEngine4B20 initializes byte40 (show) to1.
            visible: true,
            alpha: false,
            brightness: 255,
            draw_method: 0,
            surface: None,
            text_home: [0, 0],
            text_position: [0, 0],
            text_char_space: 0,
            text_line_space: 0,
            text_line_height: 0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Sprites {
    owner: ClientId,
    values: BTreeMap<i32, Sprite>,
    window: Sprite,
    capacity: i32,
}
fn layer(number: i32) -> Result<u32> {
    ensure!((-1..65536).contains(&number), "System4 sprite index budget");
    Ok(if number == -1 {
        crate::sact::MAIN_SPRITE
    } else {
        0x4100_0000 + number as u32
    })
}
impl Sprite {
    pub(crate) fn layer(&self, number: i32) -> Result<Option<Value>> {
        ensure!(
            self.size.iter().all(|n| *n <= 8192)
                && (self.surface.is_none() || self.size.iter().all(|n| *n > 0)),
            "System4 sprite surface dimensions"
        );
        // Original SactEngine 5870 selects Graph RGB tables before its normal
        // alpha path. Screen ignores brightness and source alpha; blend_rate is
        // a floor /255 source prequantization. Multiply's nonopaque base-pixel
        // branch is explicitly absent. Retained text children remain present.
        let skip_pixels = self.draw_method == 2 && self.opacity != 255;
        let blend = if self.surface.is_none() || skip_pixels {
            json!("normal")
        } else {
            match self.draw_method {
                0 => json!("normal"),
                1 | 2 => json!({"stored_byte":{
                    "mode":if self.draw_method==1 {"opaque_screen255"} else {"opaque_multiply255"}
                }}),
                _ => anyhow::bail!("unknown native sprite draw method {}", self.draw_method),
            }
        };
        let paint = match &self.surface {
            _ if skip_pixels => json!({"kind":"solid","color":[0,0,0,0]}),
            Some(Surface::Solid(color)) => json!({"kind":"solid","color":color}),
            Some(Surface::Image(path)) => {
                json!({"kind":"image","asset":path,"tint":[255,255,255,255]})
            }
            // Native custom sprites retain position/show/opacity and may own
            // independent glyph children even with no base pixel surface.
            None => json!({"kind":"solid","color":[0,0,0,0]}),
        };
        let width = self.size[0].max(1);
        let height = self.size[1].max(1);
        //Native sprite metadata and its retained text are part of the
        //scene, not the physical WindowSurface presentation layer16.
        let parent = Some(crate::sact::BACK_SCENE);
        // F080 -> engine7F20/get6FE0 owns field+32 (blend rate).
        // F130 -> engine7FA0/get7000 owns field+36 (brightness). 5870 uses
        // +32 for RGB screen strength and all glyph children; +36 affects
        // only normal base pixels. Do not exchange their equal255 defaults.
        let opacity = self.opacity;
        let brightness = if self.draw_method == 0 {
            f64::from(self.brightness) / 255.0
        } else {
            1.0
        };
        Ok(Some(
            json!({"op":"put","layer":{"id":layer(number)?,"parent":parent,"rect":[self.position[0],self.position[1],width,height],"paint":paint,"order":[self.z,number],"visible":self.visible,"opacity":opacity,"blend":blend,"filter":{"multiply":[brightness,brightness,brightness]}}}),
        ))
    }
}
impl Sprites {
    /// Install scalar native policy after the caller's one shared parent/child
    /// batch succeeded. All fallible geometry validation precedes submission.
    pub(crate) fn install_prepared(&mut self, number: i32, sprite: Sprite) {
        debug_assert!((-1..65536).contains(&number));
        self.values.insert(number, sprite);
        if number >= self.capacity {
            self.capacity = (number + 256).min(65536);
        }
    }
    pub(crate) fn owned_layer_ids(&self) -> Result<Vec<u32>> {
        self.values.keys().map(|number| layer(*number)).collect()
    }

    pub(crate) fn require_owner(&self, owner: ClientId) -> Result<()> {
        ensure!(owner == self.owner, "foreign System4 sprite owner");
        Ok(())
    }
    pub fn new(owner: ClientId, size: [u32; 2]) -> Result<Self> {
        ensure!(
            size.iter().all(|n| (1..=8192).contains(n)),
            "System4 sprite screen dimensions"
        );
        let window = Sprite {
            size,
            surface: Some(Surface::Solid([0, 0, 0, 255])),
            alpha: true,
            visible: false,
            ..Sprite::default()
        };
        Ok(Self {
            owner,
            values: BTreeMap::new(),
            window,
            capacity: 256,
        })
    }
    ///The native graphics provider7A60 resolves-1 to the physical window,
    ///independently of a possible negative entry in the ordinary sprite map.
    pub(crate) fn graphics(&self, number: i32) -> Option<&Sprite> {
        if number == -1 {
            Some(&self.window)
        } else {
            self.values.get(&number)
        }
    }
    pub(crate) fn install_window_data(&mut self, size: [u32; 2], path: String) {
        self.window.size = size;
        self.window.surface = Some(Surface::Image(path));
    }
    pub fn get(&self, number: i32) -> Option<&Sprite> {
        self.values.get(&number)
    }
    pub fn count(&self) -> i32 {
        self.values.len() as i32
    }
    pub fn max_z(&self) -> i32 {
        self.values
            .iter()
            .filter(|(n, _)| **n >= 0)
            .map(|(_, v)| v.z)
            .max()
            .unwrap_or(0)
            .max(0)
    }
    pub fn numbers(&self) -> impl Iterator<Item = i32> + '_ {
        self.values.keys().copied()
    }
    pub fn unused(&mut self, min: i32) -> Result<i32> {
        ensure!(
            (0..65536).contains(&min),
            "System4 minimum sprite index budget"
        );
        for n in min..self.capacity {
            if !self.values.contains_key(&n) {
                return Ok(n);
            }
        }
        let n = min.max(self.capacity);
        ensure!(n < 65536, "System4 sprite table exhausted");
        self.capacity = (n + 256).min(65536);
        Ok(n)
    }
    /// Commit host state first. Failed shared publication cannot acknowledge a
    /// native property change or leave a half-updated native handle table.
    pub fn update(
        &mut self,
        client: ClientId,
        number: i32,
        host: &mut dyn HostServices,
        f: impl FnOnce(&mut Sprite) -> Result<()>,
    ) -> Result<i32> {
        ensure!(client == self.owner, "foreign System4 sprite owner");
        layer(number)?;
        let mut next = self.values.get(&number).cloned().unwrap_or_default();
        f(&mut next)?;
        if let Some(op) = next.layer(number)? {
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[op]}))?)?;
        }
        self.values.insert(number, next);
        if number >= self.capacity {
            self.capacity = (number + 256).min(65536);
        }
        Ok(1)
    }
    pub fn create(
        &mut self,
        client: ClientId,
        number: i32,
        size: [u32; 2],
        color: [i32; 4],
        host: &mut dyn HostServices,
    ) -> Result<i32> {
        ensure!(
            size.iter().all(|n| (1..=8192).contains(n)),
            "System4 sprite allocation budget"
        );
        self.update(client, number, host, |sp| {
            sp.size = size;
            sp.alpha = color[3] >= 0;
            sp.surface = Some(Surface::Solid([
                color[0] as u8,
                color[1] as u8,
                color[2] as u8,
                if color[3] >= 0 { color[3] as u8 } else { 255 },
            ]));
            Ok(())
        })
    }
    pub fn image(
        &mut self,
        client: ClientId,
        number: i32,
        path: String,
        size: [u32; 2],
        has_alpha: bool,
        host: &mut dyn HostServices,
    ) -> Result<i32> {
        self.update(client, number, host, |sp| {
            sp.size = size;
            sp.alpha = has_alpha;
            sp.surface = Some(Surface::Image(path));
            Ok(())
        })
    }
    pub fn delete(
        &mut self,
        client: ClientId,
        number: i32,
        host: &mut dyn HostServices,
    ) -> Result<i32> {
        ensure!(client == self.owner, "foreign System4 sprite owner");
        if !self.values.contains_key(&number) {
            return Ok(0);
        }
        host.submit_engine(
            client,
            1,
            &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":layer(number)?}]}))?,
        )?;
        self.values.remove(&number);
        Ok(1)
    }
    pub fn scalar(
        &mut self,
        client: ClientId,
        name: &[u8],
        args: &[i32],
        host: &mut dyn HostServices,
    ) -> Result<Option<i32>> {
        ensure!(client == self.owner, "foreign System4 sprite owner");
        let get = |n: i32, f: fn(&Sprite) -> i32| self.get(n).map(f).unwrap_or(0);
        let value = match (name, args) {
            (b"SP_Count", []) => self.count(),
            (b"SP_GetMaxZ", []) => self.max_z(),
            (b"SP_IsUsing", [n]) => i32::from(self.get(*n).is_some()),
            (b"SP_ExistAlpha", [n]) => get(*n, |s| i32::from(s.alpha)),
            (b"SP_GetPosX", [n]) => get(*n, |s| s.position[0]),
            (b"SP_GetPosY", [n]) => get(*n, |s| s.position[1]),
            (b"SP_GetWidth", [n]) => get(*n, |s| s.size[0] as i32),
            (b"SP_GetHeight", [n]) => get(*n, |s| s.size[1] as i32),
            (b"SP_GetZ", [n]) => get(*n, |s| s.z),
            (b"SP_GetBlendRate", [n]) => get(*n, |s| i32::from(s.opacity)),
            (b"SP_GetShow", [n]) => get(*n, |s| i32::from(s.visible)),
            (b"SP_GetBrightness", [n]) => get(*n, |s| i32::from(s.brightness)),
            (b"SP_GetTextHomeX", [n]) => get(*n, |s| s.text_home[0]),
            (b"SP_GetTextHomeY", [n]) => get(*n, |s| s.text_home[1]),
            (b"SP_GetTextPosX", [n]) => get(*n, |s| s.text_position[0]),
            (b"SP_GetTextPosY", [n]) => get(*n, |s| s.text_position[1]),
            (b"SP_GetTextCharSpace", [n]) => get(*n, |s| s.text_char_space),
            (b"SP_GetTextLineSpace", [n]) => get(*n, |s| s.text_line_space),
            (b"SP_GetUnuseNum", [min]) => self.unused(*min)?,
            (b"SP_Create", [n, w, h, r, g, b, a]) => self.create(
                client,
                *n,
                [(*w).try_into()?, (*h).try_into()?],
                [*r, *g, *b, *a],
                host,
            )?,
            (b"SP_CreatePixelOnly", [n, w, h]) => self.create(
                client,
                *n,
                [(*w).try_into()?, (*h).try_into()?],
                [0, 0, 0, -1],
                host,
            )?,
            (b"SP_CreateCustom", [n]) => self.update(client, *n, host, |_| Ok(()))?,
            (b"SP_Delete", [n]) => self.delete(client, *n, host)?,
            (b"SP_SetPos", [n, x, y]) => self.update(client, *n, host, |s| {
                s.position = [*x, *y];
                Ok(())
            })?,
            (b"SP_SetX", [n, x]) => self.update(client, *n, host, |s| {
                s.position[0] = *x;
                Ok(())
            })?,
            (b"SP_SetY", [n, y]) => self.update(client, *n, host, |s| {
                s.position[1] = *y;
                Ok(())
            })?,
            (b"SP_SetZ", [n, z]) => self.update(client, *n, host, |s| {
                s.z = *z;
                Ok(())
            })?,
            (b"SP_SetBlendRate", [n, a]) => {
                // Original SACT F080 rejects signed negatives/outside255 and
                // n<1 before the lazy engine7F20 declaration is reached.
                if *n<1 || !(0..=255).contains(a) {0}else{
                    self.update(client,*n,host,|s|{s.opacity=*a as u8;Ok(())})?
                }
            },
            (b"SP_SetShow", [n, show]) => {
                //Original F1E0 guards this one entry, unlike SP_Create and
                //SP_SetCGFromFile. Do not forbid negative map keys globally.
                if *n < 1 {
                    0
                } else {
                    self.update(client, *n, host, |s| {
                        s.visible = *show != 0;
                        Ok(())
                    })?
                }
            }
            (b"SP_SetBrightness", [n, b]) => {
                // F130 has the same range guard; engine7FA0 calls5750(n,1)
                // and therefore creates a real metadata record when absent.
                if *n<1 || !(0..=255).contains(b) {0}else{
                    self.update(client,*n,host,|s|{s.brightness=*b as u8;Ok(())})?
                }
            }
            (b"SP_SetTextHome", [n, x, y]) => self.update(client, *n, host, |s| {
                s.text_home = [*x, *y];
                Ok(())
            })?,
            (b"SP_SetTextPos", [n, x, y]) => self.update(client, *n, host, |s| {
                if s.text_position[1] != *y {
                    s.text_line_height = 0;
                }
                s.text_position = [*x, *y];
                Ok(())
            })?,
            (b"SP_SetTextCharSpace", [n, px]) => self.update(client, *n, host, |s| {
                s.text_char_space = *px;
                Ok(())
            })?,
            (b"SP_SetTextLineSpace", [n, px]) => self.update(client, *n, host, |s| {
                s.text_line_space = *px;
                Ok(())
            })?,
            (b"SP_TextHome", [n, size]) => self.update(client, *n, host, |s| {
                s.text_position = s.text_home;
                s.text_line_height = *size;
                Ok(())
            })?,
            (b"SP_TextNewLine", [n, size]) => self.update(client, *n, host, |s| {
                let size = if *size == 0 {
                    s.text_line_height
                } else {
                    *size
                };
                s.text_position = [
                    s.text_home[0],
                    s.text_position[1]
                        .checked_add(size)
                        .and_then(|n| n.checked_add(s.text_line_space))
                        .context("System4 text newline overflow")?,
                ];
                s.text_line_height = 0;
                Ok(())
            })?,
            (b"SP_GetDrawMethod", [n]) => get(*n, |s| s.draw_method),
            (b"SP_SetDrawMethod", [n, method]) => {
                ensure!(
                    (0..=2).contains(method),
                    "unknown System4 sprite draw method {method}"
                );
                if *n < 1 {
                    0
                } else {
                    self.update(client, *n, host, |s| {
                        s.draw_method = *method;
                        Ok(())
                    })?
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
    }
}
