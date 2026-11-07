// Copyright (c) 2026 CrySirror contributors. MIT.
//! Reviewed original syscall policy. Shared Canvas owns page lifetime; the
//! engine clock owns waits. No original device, renderer or timer is hosted.
use super::*;

pub(super) struct NativeWait {
    pub pc: u32,
    pub deadline: Option<u64>,
}
impl MajiroVm {
    pub(super) fn native_wait(
        &mut self,
        pc: u32,
        arguments: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(arguments.len() == 1, "Majiro wait ABI");
        let duration = arguments[0].integer()?;
        //410060/CF35F0E3: -2 ends this native update and resumes on the next;
        //other negative values park this native frame indefinitely. Callback
        //frames may run above it, and thread teardown cancels the receipt.
        ensure!(
            duration <= 43_200_000,
            "Majiro long native wait profile is not connected"
        );
        if let Some(wait) = &self.wait {
            ensure!(
                wait.pc == pc,
                "Majiro wait receipt belongs to another instruction"
            );
            if wait
                .deadline
                .is_none_or(|deadline| host.now_ms() < deadline)
            {
                return Ok(None);
            }
            self.wait = None;
            return Ok(Some(if returns {
                vec![Value::Integer(0)]
            } else {
                vec![]
            }));
        }
        let deadline = if duration < 0 && duration != -2 {
            None
        } else {
            Some(
                host.now_ms()
                    .checked_add(duration.max(0) as u64)
                    .context("Majiro native wait clock overflow")?,
            )
        };
        self.wait = Some(NativeWait { pc, deadline });
        //Even zero/-2 yields once. The pending Native event retains all
        //arguments/PC until an actual subsequent owner update observes expiry.
        Ok(None)
    }
    pub(super) fn native_page_size(
        &self,
        arguments: &[Value],
        vertical: bool,
        returns: bool,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(arguments.len() == 1, "Majiro page_len ABI");
        let id = u32::try_from(arguments[0].integer()?).context("Majiro page handle")?;
        let size = self
            .pages
            .get(&id)
            .context("Majiro page handle is missing or foreign")?;
        //41AD40/41AD80 check CPicture then return its real width/height.
        Ok(Some(if returns {
            vec![Value::Integer(size[usize::from(vertical)] as i32)]
        } else {
            vec![]
        }))
    }
    pub(super) fn native_page_release(
        &mut self,
        arguments: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(arguments.len() == 1, "Majiro page_release ABI");
        let id = u32::try_from(arguments[0].integer()?).context("Majiro page handle")?;
        self.page_available(id)?;
        ensure!(
            self.pages.contains_key(&id),
            "Majiro release of missing or foreign page"
        );
        //4198C0 refuses pages attached as a native sprite's source or target.
        ensure!(
            !self
                .sprites
                .values()
                .any(|sprite| sprite.page == id || sprite.destination == Some(id)),
            "Majiro page is attached to a sprite"
        );
        ensure!(
            !self
                .page_bitmaps
                .values()
                .any(|page| page.alpha == page_bitmaps::Alpha::Linked(id)),
            "Majiro page is borrowed as an alpha plane"
        );
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"canvases":[{"op":"destroy","id":id}]}))?,
        )
        .map_err(core_error)?;
        self.api_calls += 1;
        self.pages.remove(&id);
        self.page_bitmaps.remove(&id);
        if let Some(path) = self.page_resources.remove(&id) {
            host.release_image_resource(self.client, &path)
                .map_err(core_error)?;
        }
        Ok(Some(if returns {
            vec![Value::Integer(0)]
        } else {
            vec![]
        }))
    }
}
