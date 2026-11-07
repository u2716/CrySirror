// GPL-2.0-or-later. Native semantics: pinned DrawGraph.c/draw.c, Nunuhara
// Cabbage; reviewed original DrawGraph wrappers. Rust (C)2026 contributors.
//! Only native rectangle/channel policy. Shared image jobs own all pixel work.
use super::*;
enum Phase {
    Source(ImageJob),
    Composite(ImageJob),
}
pub(super) struct Run {
    call: ExternalCall,
    number: i32,
    base: ImageInput,
    size: [u32; 2],
    position: [i32; 2],
    math: ImageCompositeMath,
    phase: Phase,
}
impl Run {
    pub(super) fn recognizes(name: &[u8]) -> bool {
        matches!(
            name,
            b"CopyReverseLR"
                | b"CopyReverseUD"
                | b"CopyReverseAMapLR"
                | b"CopyReverseAMapUD"
                | b"CopyReverseLRWithAMap"
                | b"CopyReverseUDWithAMap"
        )
    }
    pub(super) fn begin(
        owner: ClientId,
        call: &ExternalCall,
        name: &[u8],
        args: &[Argument],
        bridge: &Bridge,
        cg: &NativeCg,
        images: &dyn ImageMetadata,
    ) -> Result<Self> {
        let values = args
            .iter()
            .map(|a| match a {
                Argument::Scalar { kind: 10, bits } => Ok(*bits),
                _ => bail!("native filtered copy scalar ABI"),
            })
            .collect::<Result<Vec<_>>>()?;
        let [dst, dx, dy, src, sx, sy, w, h] = values.as_slice() else {
            bail!("native filtered copy rectangle ABI")
        };
        ensure!(
            (1..=8192).contains(w) && (1..=8192).contains(h),
            "native filtered copy dimensions"
        );
        let sprites = bridge
            .sact
            .as_ref()
            .and_then(|s| s.sprites.as_ref())
            .context("native filtered copy before SACT")?;
        let source = sprites
            .graphics(*src)
            .context("native filtered copy source absent")?;
        // The reviewed in-bounds cell profile never substitutes zero padding
        // for original source clipping. Further historical clip variants fail.
        ensure!(
            *sx >= 0
                && *sy >= 0
                && i64::from(*sx) + i64::from(*w) <= i64::from(source.size[0])
                && i64::from(*sy) + i64::from(*h) <= i64::from(source.size[1]),
            "native filtered copy source clipping profile"
        );
        let alpha = matches!(
            name,
            b"CopyReverseAMapLR"
                | b"CopyReverseAMapUD"
                | b"CopyReverseLRWithAMap"
                | b"CopyReverseUDWithAMap"
        );
        if alpha {
            ensure!(
                source.alpha && sprites.graphics(*dst).is_some_and(|s| s.alpha),
                "native filtered copy alpha plane absent"
            );
        }
        let size = [*w as u32, *h as u32];
        let cropped = composite(
            ImageInput::Solid {
                size,
                color: [0, 0, 0, 0],
            },
            cg.sprite_input(owner, *src, bridge)?,
            [*sx, *sy, *w, *h],
            [0, 0],
            ImageCompositeMath::Copy,
        );
        let horizontal = matches!(
            name,
            b"CopyReverseLR" | b"CopyReverseAMapLR" | b"CopyReverseLRWithAMap"
        );
        let effects = serde_json::to_vec(
            &serde_json::json!([{"kind":"flip","horizontal":horizontal,"vertical":!horizontal}]),
        )?;
        let phase = Phase::Source(images.begin_process(cropped, effects.into())?);
        let math = if matches!(name, b"CopyReverseAMapLR" | b"CopyReverseAMapUD") {
            ImageCompositeMath::CopyAlpha
        } else if alpha {
            ImageCompositeMath::Copy
        } else {
            ImageCompositeMath::CopyRgb
        };
        Ok(Self {
            call: call.clone(),
            number: *dst,
            base: cg.sprite_input(owner, *dst, bridge)?,
            size,
            position: [*dx, *dy],
            math,
            phase,
        })
    }
    pub(super) fn poll(
        &mut self,
        owner: ClientId,
        call: &ExternalCall,
        machine: &mut Machine,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        host: &mut dyn HostServices,
        images: &dyn ImageMetadata,
    ) -> Result<Dispatch> {
        ensure!(*call == self.call, "stale native filtered copy completion");
        if let Phase::Source(job) = &mut self.phase {
            match job.poll()? {
                ImageProcessState::Pending => {
                    return Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1)))
                }
                ImageProcessState::Failed(e) => bail!("native shared filtered source: {e}"),
                ImageProcessState::Ready(resource) => {
                    ensure!(
                        resource.client() == owner && resource.size() == self.size,
                        "native filtered source owner/size"
                    );
                    self.phase = Phase::Composite(images.begin_process(
                        composite(
                            self.base.clone(),
                            ImageInput::Resource(resource),
                            [0, 0, self.size[0] as i32, self.size[1] as i32],
                            self.position,
                            self.math,
                        ),
                        b"[]".as_slice().into(),
                    )?);
                }
            }
        }
        let Phase::Composite(job) = &mut self.phase else {
            unreachable!()
        };
        match job.poll()? {
            ImageProcessState::Pending => Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1))),
            ImageProcessState::Failed(e) => bail!("native shared filtered destination: {e}"),
            ImageProcessState::Ready(resource) => {
                cg.publish_draw_result(owner, self.number, resource, bridge, host)?;
                machine.complete_hll(call, HllReturn::Void)?;
                Ok(Dispatch::Completed)
            }
        }
    }
}
