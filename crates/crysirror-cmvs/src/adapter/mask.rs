// SPDX-License-Identifier: MPL-2.0
//! Native mask lifetime and wait policy over the shared Layer coverage filter.

use super::visual::{DecodedImage, Objects};
use crate::vm::CmvsVm;
use anyhow::{ensure, Result};
use crysirror_core::{
    service::{HostServices, UploadedImage},
    state::ClientId,
};
use serde_json::{json, Value};

pub(super) struct Request {
    pub name: String,
    pub slot: usize,
    pub channel: usize,
    duration: u32,
    pub reveal: bool,
    /// Argument words the completing selector reclaims.
    pub words: u16,
    /// `0x222e`: the target is the captured screen in object 0.
    pub screen: bool,
}

impl Request {
    pub fn read(vm: &CmvsVm, opcode: u16) -> Result<Self> {
        let [name, slot, duration, _edge, mode] = vm.owner_arguments(opcode)?;
        ensure!(slot < 256, "CMVS mask target slot");
        Ok(Self {
            name: vm.image_name(opcode, name)?,
            slot: slot as usize,
            channel: usize::from(opcode == 0x2234),
            duration,
            reveal: mode != 0,
            words: 5,
            screen: false,
        })
    }

    /// `0x222e` (0x140088200): a hiding mask over the captured screen in object 0.
    pub fn screen(vm: &CmvsVm, opcode: u16) -> Result<Self> {
        let [name, duration, _edge] = vm.owner_arguments(opcode)?;
        Ok(Self {
            name: vm.image_name(opcode, name)?,
            slot: 0,
            channel: 0,
            duration,
            reveal: false,
            words: 3,
            screen: true,
        })
    }
}

#[derive(Clone)]
pub(super) struct Sample {
    asset: UploadedImage,
    progress: f32,
}

impl Sample {
    pub fn filter(&self) -> Value {
        json!({"mask":{"asset":self.asset.path.relative(),"channel":"green",
            "coverage":{"progress":self.progress,"feather":0.5}}})
    }

    pub fn asset(&self) -> &str {
        self.asset.path.relative()
    }
}

#[derive(Clone)]
pub(super) struct Active {
    slot: usize,
    duration: u32,
    elapsed: u32,
    reveal: bool,
    sample: Sample,
}

impl Active {
    pub fn target(&self) -> usize {
        self.slot
    }

    pub fn remaining(&self) -> u32 {
        self.duration.saturating_sub(self.elapsed)
    }

    pub fn begin(
        state: &mut Option<Self>,
        objects: &mut Objects,
        host: &mut dyn HostServices,
        client: ClientId,
        request: Request,
        decoded: DecodedImage,
    ) -> Result<()> {
        let size = objects
            .image_size(request.slot)
            .ok_or_else(|| anyhow::anyhow!("CMVS mask target has no image"))?;
        ensure!(
            size == decoded.size(),
            "CMVS mask dimensions differ from target"
        );
        Self::remove(state, objects, host, client, false)?;
        let asset = decoded.publish(host, client, 4)?;
        let mask = Self {
            slot: request.slot,
            duration: request.duration,
            elapsed: 0,
            reveal: request.reveal,
            sample: Sample {
                asset,
                progress: f32::from(request.reveal == (request.duration == 0)),
            },
        };
        if let Err(error) = objects.mask(host, client, request.slot, Some(mask.sample.clone())) {
            host.release_image_resource(client, mask.sample.asset())?;
            return Err(error);
        }
        *state = Some(mask);
        Ok(())
    }

    pub fn advance(
        state: &mut Option<Self>,
        objects: &mut Objects,
        host: &mut dyn HostServices,
        client: ClientId,
        delta: u32,
    ) -> Result<()> {
        let Some(mask) = state else {
            return Ok(());
        };
        let elapsed = mask.elapsed.wrapping_add(delta);
        if elapsed >= mask.duration {
            return Self::remove(state, objects, host, client, true);
        }
        let progress = elapsed as f32 / mask.duration as f32;
        let mut sample = mask.sample.clone();
        sample.progress = if mask.reveal { progress } else { 1. - progress };
        objects.mask(host, client, mask.slot, Some(sample.clone()))?;
        mask.sample = sample;
        mask.elapsed = elapsed;
        Ok(())
    }

    pub fn remove(
        state: &mut Option<Self>,
        objects: &mut Objects,
        host: &mut dyn HostServices,
        client: ClientId,
        finish: bool,
    ) -> Result<()> {
        if let Some(mask) = state {
            if finish && !mask.reveal {
                objects.reset(host, client, mask.slot, None, false)?;
            } else {
                objects.mask(host, client, mask.slot, None)?;
            }
            host.release_image_resource(client, mask.sample.asset())?;
            *state = None;
        }
        Ok(())
    }
}
