// Copyright (c) 2026 CrySirror contributors. MIT.
//! 43DBE0/43DD80 sprite descriptors over shared Layer/Image resources.
//! Construction does not draw: native output positions start at100000 while
//! the logical position accessors initially report zero. Draw/animation policy
//! is admitted separately; creation must not flash a sprite at the origin.
use super::*;
pub(super) struct Sprite {
    pub page: u32,
    pub source: [u32; 4],
    pub destination: Option<u32>,
    pub owns_page: bool,
    pub group: u32,
    pub alpha: u8,
}
impl MajiroVm {
    pub(super) fn sprite_preflight(&self, args: &[Value]) -> Result<Option<u32>> {
        ensure!(
            matches!(args.len(), 1 | 5 | 6),
            "Majiro sprite_create_file ABI"
        );
        ensure!(
            self.sprites.len() < 4096 && self.next_sprite < 0x2000_0000,
            "Majiro sprite handle budget"
        );
        if args.len() >= 5 {
            let values = args[1..5]
                .iter()
                .map(Value::integer)
                .collect::<Result<Vec<_>>>()?;
            ensure!(
                values[0] >= 0 && values[1] >= 0 && values[2] > 0 && values[3] > 0,
                "Majiro sprite source rectangle profile"
            );
        }
        if args.len() == 6 {
            let id = u32::try_from(args[5].integer()?).context("Majiro sprite destination page")?;
            ensure!(
                self.pages.contains_key(&id),
                "Majiro sprite destination is missing or foreign"
            );
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }
    pub(super) fn sprite_resource_layer(
        &self,
        args: &[Value],
        size: [u32; 2],
        path: &str,
        page: u32,
        owns_page: bool,
    ) -> Result<(serde_json::Value, Sprite)> {
        let destination = self.sprite_preflight(args)?;
        let source = if args.len() >= 5 {
            [
                args[1].integer()? as u32,
                args[2].integer()? as u32,
                args[3].integer()? as u32,
                args[4].integer()? as u32,
            ]
        } else {
            [0, 0, size[0], size[1]]
        };
        ensure!(
            source[0]
                .checked_add(source[2])
                .is_some_and(|x| x <= size[0])
                && source[1]
                    .checked_add(source[3])
                    .is_some_and(|y| y <= size[1]),
            "Majiro sprite source view exceeds its page"
        );
        let uv = [
            source[0] as f32 / size[0] as f32,
            source[1] as f32 / size[1] as f32,
            (source[0] + source[2]) as f32 / size[0] as f32,
            (source[1] + source[3]) as f32 / size[1] as f32,
        ];
        let layer = json!({"op":"put","layer":{"id":self.next_sprite,"rect":[0,0,source[2],source[3]],"order":[self.next_sprite,0],
            "visible":false,"paint":{"kind":"image","asset":path},"uv":uv}});
        Ok((
            layer,
            Sprite {
                page,
                source,
                destination,
                owns_page,
                group: self.next_sprite,
                alpha: 0,
            },
        ))
    }
    pub(super) fn create_page_sprite(
        &mut self,
        args: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        self.sprite_preflight(args)?;
        let page = u32::try_from(args[0].integer()?).context("Majiro sprite source page")?;
        self.page_available(page)?;
        let size = *self
            .pages
            .get(&page)
            .context("Majiro sprite source page missing or foreign")?;
        let path = self
            .page_resources
            .get(&page)
            .context("Majiro sprite page has no published bitmap")?;
        let (layer, state) = self.sprite_resource_layer(args, size, path, page, false)?;
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"layers":[layer]}))?,
        )
        .map_err(core_error)?;
        let id = self.next_sprite;
        self.next_sprite += 1;
        self.sprites.insert(id, state);
        self.sprite_order.insert(0, id);
        self.api_calls += 1;
        Ok(Some(if returns {
            vec![Value::Integer(id as i32)]
        } else {
            vec![]
        }))
    }
    pub(super) fn sprite_alpha(
        &mut self,
        args: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(matches!(args.len(), 2 | 3), "Majiro sprite_alfa_set ABI");
        let id = u32::try_from(args[0].integer()?).context("Majiro sprite handle")?;
        ensure!(
            self.sprites.contains_key(&id),
            "Majiro sprite missing or foreign"
        );
        let alpha = u8::try_from(args[1].integer()?).context("Majiro sprite transparency byte")?;
        ensure!(
            args.len() == 2 || args[2].integer()? == 0,
            "Majiro timed sprite alpha requires reviewed native interpolation"
        );
        //43EBB0's immediate branch stores native transparency;43C790 skips255.
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(
                &json!({"layers":[{"op":"set_opacity","id":id,"opacity":255-alpha}]}),
            )?,
        )
        .map_err(core_error)?;
        self.sprites.get_mut(&id).unwrap().alpha = alpha;
        self.api_calls += 1;
        Ok(Some(if returns {
            vec![Value::Integer(0)]
        } else {
            vec![]
        }))
    }
    pub(super) fn sprite_priority(
        &mut self,
        hash: u32,
        args: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(matches!(args.len(), 1 | 2), "Majiro sprite priority ABI");
        let id = u32::try_from(args[0].integer()?).context("Majiro sprite handle")?;
        ensure!(
            self.sprites.contains_key(&id),
            "Majiro priority sprite missing or foreign"
        );
        let high = matches!(hash, 0x4a02d664 | 0x5548ef5e);
        let single = matches!(hash, 0x5548ef5e | 0x7bffc5b1);
        let mut order = self.sprite_order.clone();
        order.retain(|sprite| *sprite != id);
        let mut group = None;
        let at = if args.len() == 2 {
            let reference =
                u32::try_from(args[1].integer()?).context("Majiro priority reference")?;
            let target = self
                .sprites
                .get(&reference)
                .context("Majiro priority reference missing or foreign")?;
            //The native linked-list helpers can form a self-cycle for this
            //input. Define self-relative ordering as preserving the current
            //order rather than reproducing pointer corruption.
            if reference == id {
                return Ok(Some(if returns {
                    vec![Value::Integer(0)]
                } else {
                    vec![]
                }));
            }
            let mut at = order
                .iter()
                .position(|sprite| *sprite == reference)
                .context("Majiro sprite order reference absent")?;
            if !single {
                if high {
                    while at > 0 && self.sprites[&order[at - 1]].group == target.group {
                        at -= 1;
                    }
                } else {
                    while at + 1 < order.len() && self.sprites[&order[at + 1]].group == target.group
                    {
                        at += 1;
                    }
                }
            }
            group = Some(target.group);
            if high { at } else { at + 1 }
        } else if high {
            0
        } else {
            order.len()
        };
        order.insert(at, id);
        let commands = order
            .iter()
            .enumerate()
            .map(|(at, id)| json!({"op":"set_order","id":id,"order":[order.len()-at,0]}))
            .collect::<Vec<_>>();
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"layers":commands}))?,
        )
        .map_err(core_error)?;
        if let Some(group) = group {
            self.sprites.get_mut(&id).unwrap().group = group;
        }
        self.sprite_order = order;
        self.api_calls += 1;
        Ok(Some(if returns {
            vec![Value::Integer(0)]
        } else {
            vec![]
        }))
    }
    pub(super) fn sprite_query(
        &self,
        hash: u32,
        args: &[Value],
        returns: bool,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(args.len() == 1, "Majiro sprite query ABI");
        let id = u32::try_from(args[0].integer()?).context("Majiro sprite handle")?;
        let sprite = self.sprites.get(&id);
        let value = if hash == 0xdcc706ef {
            i32::from(sprite.is_some())
        } else {
            let sprite = sprite.context("Majiro missing or foreign sprite")?;
            match hash {
                0x265e07d7 => sprite.page as i32,
                //43F600/43F660: constructors use the unmodified logical
                //position path (18+20/19+21), initially0, not hidden100000.
                0x6b4a7873 | 0xf6459905 => 0,
                _ => bail!("Majiro unsupported sprite query"),
            }
        };
        Ok(Some(if returns {
            vec![Value::Integer(value)]
        } else {
            vec![]
        }))
    }
    pub(super) fn release_sprite(
        &mut self,
        args: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(args.len() == 1, "Majiro sprite_release ABI");
        let id = u32::try_from(args[0].integer()?).context("Majiro sprite handle")?;
        let sprite = self
            .sprites
            .get(&id)
            .context("Majiro missing or foreign sprite")?;
        let page = sprite.page;
        let owns_page = sprite.owns_page;
        self.page_available(page)?;
        ensure!(
            !owns_page
                || !self
                    .page_bitmaps
                    .values()
                    .any(|bitmap| bitmap.alpha == page_bitmaps::Alpha::Linked(page)),
            "Majiro sprite-owned page is borrowed as an alpha plane"
        );
        ensure!(
            !owns_page
                || !self.sprites.iter().any(|(other, sprite)| *other != id
                    && (sprite.page == page || sprite.destination == Some(page))),
            "Majiro sprite-owned page is referenced by another sprite"
        );
        //File constructors own their page (offset32=1);43DAC0 borrows it.
        //All admitted sprites have no transform/animation allocation.
        let canvases = if owns_page {
            vec![json!({"op":"destroy","id":page})]
        } else {
            vec![]
        };
        let batch = json!({"layers":[{"op":"remove","id":id}],"canvases":canvases});
        host.submit_engine(self.client, 1, &serde_json::to_vec(&batch)?)
            .map_err(core_error)?;
        self.api_calls += 1;
        self.sprites.remove(&id);
        self.sprite_order.retain(|sprite| *sprite != id);
        if owns_page {
            self.pages.remove(&page);
            self.page_bitmaps.remove(&page);
            if let Some(path) = self.page_resources.remove(&page) {
                host.release_image_resource(self.client, &path)
                    .map_err(core_error)?;
            }
        }
        Ok(Some(if returns {
            vec![Value::Integer(0)]
        } else {
            vec![]
        }))
    }
}
