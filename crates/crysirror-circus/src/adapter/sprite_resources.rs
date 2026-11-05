//! Native texture handles are shared image leases bound to independent Layers.
use super::*;
use crate::machine::sprite_ops::{SpriteResourceReply, SpriteResourceRequest};
pub(super) fn alias(object: u16) -> u32 {
    crate::machine::visual_ops::object_resource(object)
        .map_or(0x3a05_0000 + u32::from(object), |slot| {
            0x3700_0000 + u32::from(slot)
        })
}
impl Bridge<'_> {
    pub(in crate::adapter) fn native_primary_visible(&self,object:u16,visible:bool)->bool {
        visible&&!self.services.effect_control.particle_wrapper(object)
    }
    pub(super) fn prune_sprite_shader(&mut self, object: u16) -> Result<bool> {
        ensure!(object < 150, "Circus shader-queue object index");
        self.finish_sprite_mask(object)?;
        Ok(true)
    }

    pub(super) fn sprite_resource_bound(&mut self, object: u16, slot: u8) -> Result<bool> {
        ensure!(
            object < 150 && slot < 51,
            "Circus bound sprite resource index"
        );
        let image = self.services.images.get(&slot);
        let current: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::Layer(alias(object)))?,
        )?;
        if let Some(image) = image {
            Ok(current["paint"]["kind"] == "image"
                && current["paint"]["asset"] == image.path.relative())
        } else {
            let packed: Value = serde_json::from_slice(&self.host.query_engine(
                self.client,
                EngineQuery::Layer(0x3e00_0000 + u32::from(object)),
            )?)?;
            Ok(current["paint"]["kind"] != "image" && packed["paint"]["kind"] != "image")
        }
    }

    pub(super) fn native_sprite_resources(
        &mut self,
        request: &SpriteResourceRequest,
    ) -> Result<Option<SpriteResourceReply>> {
        for binding in &request.bindings {
            ensure!(
                binding.slot < 51 && binding.object < 150,
                "Circus sprite resource binding index"
            );
        }
        let mut commands = Vec::new();
        let mut dimensions = BTreeMap::new();
        let mut resources = self.services.primary.resources.clone();
        for binding in &request.bindings {
            self.visual_root(binding.object)?;
            let root = visual::content_layer(binding.object);
            let packed_layer = 0x3e00_0000 + u32::from(binding.object);
            let previous: Value = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::Layer(packed_layer))?,
            )?;
            if !previous.is_null() {
                commands.push(json!({"op":"remove","id":packed_layer}));
            }

            let Some(image) = self.services.images.get(&binding.slot).cloned() else {
                commands.push(json!({"op":"put","layer":{"id":alias(binding.object),"parent":root,"rect":[0,0,1,1],"paint":{"kind":"solid","color":[0,0,0,0]}}}));
                dimensions.insert(binding.object, None);
                resources.remove(&binding.object);
                continue;
            };
            commands.push(json!({"op":"set_visible","id":visual::object_layer(binding.object),"visible":self.native_primary_visible(binding.object,true)}));
            let logical = self
                .services
                .image_extents
                .get(&binding.slot)
                .copied()
                .context("Circus sprite resource extent missing")?;
            let crop = binding
                .crop
                .or_else(|| resources.get(&binding.object).map(|(_, crop)| *crop).filter(|crop|crop[2]!=crop[0]&&crop[3]!=crop[1]))
                .unwrap_or([0, 0, i32::try_from(logical[0])?, i32::try_from(logical[1])?]);
            let [l, t, r, b] = crop;
            let quad=crysirror_scene::layer::crop::texel_center_quad(image.size,logical.map(|value|value as f32),[l as f32,t as f32,r as f32,b as f32],[0.,0.])?;
            let size=quad.map(|quad|quad.size).unwrap_or([1.,1.]);let uv=quad.map(|quad|quad.uv);
            let filter = self
                .services
                .visual
                .filters
                .get(&binding.object)
                .cloned()
                .unwrap_or_else(|| json!({"encoded_sampling":true}));
            commands.push(json!({"op":"put","layer":{"id":alias(binding.object),"parent":root,"rect":[0,0,size[0],size[1]],"uv":uv,"visible":quad.is_some(),"sampling":{"address_u":"repeat","address_v":"repeat"},"filter":filter,"paint":{"kind":"image","asset":image.path.relative()}}}));
            dimensions.insert(binding.object, Some((image.size, logical)));
            resources.insert(binding.object, (Some(binding.slot), crop));
        }
        self.submit(commands, vec![])?;
        self.services.primary.resources = resources;
        for binding in &request.bindings {if let Some((_,crop))=self.services.primary.resources.get(&binding.object).copied(){self.set_primary_source_extent(binding.object,[crop[2] as f32-crop[0] as f32,crop[3] as f32-crop[1] as f32])?;}}
        Ok(Some(SpriteResourceReply { dimensions }))
    }
    ///443EA0 can bind a capture handle that is not one of the51 native slots.
    ///Use the same retained bitmap leaf/filter/crop without replacing slot1's
    ///original image or filename, and keep its owner distinct on later unload.
    pub(super) fn bind_transient_sprite_image(
        &mut self,
        object: u16,
        image: &UploadedImage,
        logical:[u32;2],
        crop: [i32; 4],
    ) -> Result<()> {
        self.visual_root(object)?;
        let [l, t, r, b] = crop;
        let quad=crysirror_scene::layer::crop::texel_center_quad(image.size,logical.map(|value|value as f32),[l as f32,t as f32,r as f32,b as f32],[0.,0.])?;
        let size=quad.map(|quad|quad.size).unwrap_or([1.,1.]);let uv=quad.map(|quad|quad.uv);
        let filter = self
            .services
            .visual
            .filters
            .get(&object)
            .cloned()
            .unwrap_or_else(|| json!({"encoded_sampling":true}));
        self.submit(vec![json!({"op":"set_visible","id":visual::object_layer(object),"visible":self.native_primary_visible(object,true)}),json!({"op":"put","layer":{
            "id":alias(object),"parent":visual::content_layer(object),
            "rect":[0,0,size[0],size[1]],"uv":uv,"visible":quad.is_some(),"sampling":{"address_u":"repeat","address_v":"repeat"},
            "filter":filter,"paint":{"kind":"image","asset":image.path.relative()}
        }})],vec![])?;
        self.services.primary.resources.insert(object, (None, crop));
        self.set_primary_source_extent(object,[r as f32-l as f32,b as f32-t as f32])?;
        if let Some(slot) = crate::machine::visual_ops::object_resource(object) {
            self.services.displayed.insert(slot);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        engine: crysirror_scene::EngineSystems,
    }
    impl crysirror_core::service::FileSystem for Host {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(crysirror_core::Error::Unsupported("unexpected asset read"))
        }
        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::Unsupported("unexpected write"))
        }
    }
    impl HostServices for Host {
        fn submit(
            &mut self,
            _: ClientId,
            _: &[crysirror_core::state::Command],
        ) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::Unsupported(
                "unexpected legacy command",
            ))
        }
        fn now_ms(&self) -> u64 {
            0
        }
        fn submit_engine(
            &mut self,
            client: ClientId,
            _: u32,
            bytes: &[u8],
        ) -> crysirror_core::Result<()> {
            (|| -> Result<()> {
                self.engine
                    .submit(client, &serde_json::from_slice(bytes)?)?;
                Ok(())
            })()
            .map_err(|error| crysirror_core::Error::Io(error.to_string()))
        }
        fn query_engine(
            &mut self,
            client: ClientId,
            query: EngineQuery,
        ) -> crysirror_core::Result<Vec<u8>> {
            (|| -> Result<Vec<u8>> {
                let value = match query {
                    EngineQuery::Layer(id) => {
                        serde_json::to_value(self.engine.layers.get(client, id))?
                    }
                    EngineQuery::LayerSequence(id) => {
                        serde_json::to_value(self.engine.layers.sequence_status(client, id)?)?
                    }
                    _ => bail!("unexpected query"),
                };
                Ok(serde_json::to_vec(&value)?)
            })()
            .map_err(|error| crysirror_core::Error::Io(error.to_string()))
        }
    }
    #[test]
    fn atlas_lease_binds_independent_owned_crop_layers() {
        let mut services = Services::default();
        let client = ClientId(7);
        services.images.insert(
            22,
            UploadedImage {
                path: VfsPath::new(Mount::Asset, "atlas.png").unwrap(),
                size: [2048, 1024],
                opaque: false,
            },
        );
        services.image_extents.insert(22, [1280, 720]);
        let mut host = Host::default();
        let mut bridge = Bridge {
            services: &mut services,
            host: &mut host,
            client,
            font: "test",
            input: InputFrame::default(),
        };
        let reply = bridge
            .native_sprite_resources(&SpriteResourceRequest {
                bindings: vec![
                    crate::machine::sprite_ops::SpriteResourceBinding {
                        slot: 22,
                        object: 96,
                        crop: Some([720, 0, 998, 256]),
                    },
                    crate::machine::sprite_ops::SpriteResourceBinding {
                        slot: 22,
                        object: 97,
                        crop: Some([0, 0, 732, 720]),
                    },
                ],
            })
            .unwrap()
            .unwrap();
        assert_eq!(reply.dimensions[&96], Some(([2048, 1024], [1280, 720])));
        assert!(bridge.sprite_resource_bound(96, 22).unwrap());
        let first: Value = serde_json::from_slice(
            &bridge
                .host
                .query_engine(client, EngineQuery::Layer(alias(96)))
                .unwrap(),
        )
        .unwrap();
        let second: Value = serde_json::from_slice(
            &bridge
                .host
                .query_engine(client, EngineQuery::Layer(alias(97)))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(first["rect"], json!([0., 0., 278., 256.]));
        assert_eq!(second["rect"], json!([0., 0., 732., 720.]));
        assert_eq!(first["uv"], json!([720. / 2048., 0., 998. / 2048., 0.25]));
        assert_eq!(first["paint"]["asset"], second["paint"]["asset"]);
    }
}
