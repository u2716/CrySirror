// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native ImageList draw-key/plane-view admission over engine Canvas resources.
//! This retains original IDs and resource policy, not pixels, dirty raster code
//! or another renderer. Canvas owns clipping/copy/isolation and image lifetimes.
use anyhow::{ensure, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_scene::{
    canvas::{CanvasCommand, CanvasNode, CanvasSurface},
    layer::LayerSpec,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) struct Draw {
    ///Original sender | (draw ID <<16); no host/process pointers.
    pub key: u32,
    pub image: String,
    pub size: [u32; 2],
    ///Native world position from748EF0, before the draw-context pivot.
    pub position: [f32; 2],
    pub pivot: [f32; 2],
    pub alpha: u8,
    pub visible: bool,
    ///Native7464A0 full-manager clipping is represented by None.
    pub clip: Option<[i32; 4]>,
}
impl Draw {
    ///743600/747580 mode0 does integer translation and ignores transform words.
    ///Mode1's fixed-point affine sampling is a separate reviewed API boundary;
    ///callers must not pass it through this mode0 profile silently.
    pub fn axis_aligned(&self) -> Result<LayerSpec> {
        ensure!(
            self.size.iter().all(|v| (1..=8192).contains(v)),
            "CatSystem2 native draw image extent"
        );
        ensure!(
            self.position
                .into_iter()
                .chain(self.pivot)
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
            "CatSystem2 native draw coordinates"
        );
        let position = [
            self.position[0].trunc() - self.pivot[0].trunc(),
            self.position[1].trunc() - self.pivot[1].trunc(),
        ];
        let clip = self.clip.map(|r| {
            [
                // Native clips are manager coordinates; LayerSpec.clip is rect-local.
                r[0] as f32 - position[0],
                r[1] as f32 - position[1],
                r[2].saturating_sub(r[0]) as f32,
                r[3].saturating_sub(r[1]) as f32,
            ]
        });
        if let Some(r) = self.clip {
            ensure!(
                r[0] < r[2] && r[1] < r[3],
                "CatSystem2 noncanonical native draw clip"
            );
        }
        Ok(serde_json::from_value(
            serde_json::json!({"id":self.key,"rect":[position[0],position[1],self.size[0],self.size[1]],"paint":{"kind":"image","asset":self.image},"opacity":self.alpha,"visible":self.visible,"clip":clip}),
        )?)
    }
}
#[derive(Clone, Debug)]
pub(crate) struct Plane {
    pub canvas: u32,
    pub presentation: u32,
    ///A real class-owned engine Layer. Opacity/matrix/visibility live there.
    pub parent: u32,
    ///Actual native VRAM origin and declared size, in atlas coordinates.
    pub source: [i32; 4],
}
#[derive(Clone, Debug)]
pub(crate) struct ImageList {
    client: ClientId,
    canvas: u32,
    size: [u32; 2],
    records: BTreeMap<u32, (i32, Draw)>,
    planes: BTreeMap<u32, Plane>,
    ///Native reserved bitmap0 can be a frozen engine Canvas copy. This is
    ///resource metadata; Canvas owns copying, clipping and snapshot lifetime.
    canvas_records:BTreeMap<u32,(i32,u32,[u32;2])>,
}
impl ImageList {
    pub fn new(client: ClientId, canvas: u32, size: [u32; 2]) -> Result<Self> {
        ensure!(
            size.iter().all(|v| (1..=8192).contains(v))
                && u64::from(size[0]) * u64::from(size[1]) <= 16 * 1024 * 1024,
            "CatSystem2 native image-list Canvas extent"
        );
        Ok(Self {
            client,
            canvas,
            size,
            records: BTreeMap::new(),
            planes: BTreeMap::new(),
            canvas_records:BTreeMap::new(),
        })
    }
    pub fn scope(&self) -> (ClientId, u32, [u32; 2]) {
        (self.client, self.canvas, self.size)
    }
    pub fn key(sender: u32, draw: u32) -> Result<u32> {
        ensure!(
            sender <= u16::MAX.into() && draw <= u16::MAX.into(),
            "CatSystem2 native draw-key profile"
        );
        Ok(sender | (draw << 16))
    }
    pub fn put(&mut self, mut record: Draw) -> Result<()> {
        record.axis_aligned()?;
        let key = record.key;
        ensure!(
            self.records.contains_key(&key) || self.canvas_records.contains_key(&key) || self.records.len()+self.canvas_records.len() < 4096,
            "CatSystem2 native ImageList draw budget"
        );
        //742A70 delegates updates to743230; an existing entry retains its
        //separately assigned clip until the subsequent original289 message.
        let priority = if let Some((priority, old)) = self.records.get(&key) {
            record.clip = old.clip;
            *priority
        } else {
            (key >> 16) as i32
        };
        self.records.insert(key, (priority, record));
        self.canvas_records.remove(&key);
        Ok(())
    }
    pub fn remove(&mut self, key: u32) -> bool {
        let image=self.records.remove(&key).is_some();
        self.canvas_records.remove(&key).is_some()||image
    }
    pub fn put_canvas(&mut self,key:u32,source:u32,size:[u32;2])->Result<()> {
        ensure!(source!=self.canvas&&size.iter().all(|v|(1..=8192).contains(v)),"CatSystem2 frozen native bitmap Canvas scope/extent");
        ensure!(self.records.contains_key(&key)||self.canvas_records.contains_key(&key)||self.records.len()+self.canvas_records.len()<4096,"CatSystem2 native ImageList draw budget");
        self.records.remove(&key);self.canvas_records.insert(key,((key>>16)as i32,source,size));Ok(())
    }
    /// 7464A0: a draw record's clip rectangle.
    pub fn set_clip(&mut self,key:u32,clip:Option<[i32;4]>)->bool{
        let Some((_,draw))=self.records.get_mut(&key) else{return false};
        let changed=draw.clip!=clip;draw.clip=clip;changed
    }
    /// 746460: a draw record's visibility.
    pub fn set_visible(&mut self,key:u32,visible:bool)->bool{
        let Some((_,draw))=self.records.get_mut(&key) else{return false};
        let changed=draw.visible!=visible;draw.visible=visible;changed
    }
    /// 7430C0: drop every draw record.
    pub fn clear(&mut self){self.records.clear();self.canvas_records.clear();}
    pub fn clear_sender(&mut self, sender:u32)->Result<()> {
        ensure!(sender<=u16::MAX.into(),"CatSystem2 native draw sender range");
        self.records.retain(|key,_|key&0xffff!=sender);
        self.canvas_records.retain(|key,_|key&0xffff!=sender);Ok(())
    }
    pub fn take_planes(&mut self, parents:&std::collections::BTreeSet<u32>)->Vec<Plane>{
        let keys=self.planes.keys().copied().filter(|id|parents.contains(id)).collect::<Vec<_>>();
        keys.into_iter().filter_map(|id|self.planes.remove(&id)).collect()
    }
    pub fn clip(&mut self, key: u32, rect: Option<[i32; 4]>) -> bool {
        let Some((_, draw)) = self.records.get_mut(&key) else {
            return false;
        };
        //743E40 treats a native empty RECT like nullptr: reset full-manager
        //clip. It must not become the generic engine's empty-coverage clip.
        draw.clip = rect.filter(|r| r[0] < r[2] && r[1] < r[3]);
        true
    }
    ///Original299 changes the independent signed priority, preserving permanent
    ///draw identity.745430 compares priority with signed JG/JL then the full
    ///draw-key DWORD unsigned JAE; sorting only by HIWORD(key) would be wrong.
    pub fn priority(&mut self, key: u32, priority: i32) -> bool {
        let Some((stored, _)) = self.records.get_mut(&key) else {
            return false;
        };
        *stored = priority;
        true
    }
    pub fn plane(&mut self, plane: Plane) -> Result<()> {
        ensure!(
            plane.canvas != self.canvas
                && plane.source[2] > 0
                && plane.source[3] > 0
                && plane.source[2] <= 8192
                && plane.source[3] <= 8192,
            "CatSystem2 native Plane Canvas extent"
        );
        ensure!(
            self.planes.contains_key(&plane.parent) || self.planes.len() < 4096,
            "CatSystem2 native Plane publication count"
        );
        self.planes.insert(plane.parent, plane);
        Ok(())
    }
    pub fn commands(&self) -> Result<Vec<CanvasCommand>> {
        let mut ordered = self.records.values().collect::<Vec<_>>();
        ordered.sort_by_key(|(priority, draw)| (*priority, draw.key));
        let mut commands = vec![CanvasCommand::Restore {
            id: self.canvas,
            surface: CanvasSurface {
                width: self.size[0],
                height: self.size[1],
                nodes: ordered
                    .into_iter()
                    .map(|(_, draw)| {
                        Ok(CanvasNode::Draw {
                            layer: draw.axis_aligned()?,
                        })
                    })
                    .collect::<Result<_>>()?,
            },
        }];
        if !self.canvas_records.is_empty(){
            //When key0 refers to a native copied bitmap, merge it with all
            //normal draws by the SAME signed-priority/unsigned-key ordering.
            //Each copy is frozen by the shared Canvas service, never flattened
            //or sampled by this VM adapter.
            commands.clear();commands.push(CanvasCommand::Restore{id:self.canvas,surface:CanvasSurface{width:self.size[0],height:self.size[1],nodes:Vec::new()}});
            let mut keys=self.records.iter().map(|(key,(priority,_))|(*priority,*key)).chain(self.canvas_records.iter().map(|(key,(priority,_,_))|(*priority,*key))).collect::<Vec<_>>();
            keys.sort_unstable();
            for (_,key) in keys {
                if let Some((_,draw))=self.records.get(&key){
                    commands.push(CanvasCommand::Draw{id:self.canvas,layer:draw.axis_aligned()?,atop:None});
                }else{
                    let (_,source,[w,h])=self.canvas_records[&key];
                    commands.push(CanvasCommand::Copy{source,destination:self.canvas,rect:[0.,0.,w as f32,h as f32],position:[0.;2],opacity:255,replace:false,clip:None,atop:None});
                }
            }
        }
        for plane in self.planes.values() {
            let [x, y, w, h] = plane.source;
            commands.push(CanvasCommand::Restore {
                id: plane.canvas,
                surface: CanvasSurface {
                    width: w as u32,
                    height: h as u32,
                    nodes: Vec::new(),
                },
            });
            commands.push(CanvasCommand::Copy {
                source: self.canvas,
                destination: plane.canvas,
                rect: [x as f32, y as f32, w as f32, h as f32],
                position: [0.; 2],
                opacity: 255,
                replace: true,
                clip: None,
                atop: None,
            });
            commands.push(CanvasCommand::PresentIsolated {
                id: plane.canvas,
                layer: plane.presentation,
                order: 0,
                parent: Some(plane.parent),
                position: [0.; 2],
            });
        }
        Ok(commands)
    }
    ///The caller mutates a clone; only a successful engine batch replaces the
    ///old native generation. Resource preparation must already have published
    ///every image referenced here, and native ACK still follows this receipt.
    pub fn publish(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        layers: Vec<crysirror_scene::layer::LayerCommand>,
    ) -> Result<()> {
        self.publish_prepared(host, client, Vec::new(), layers)
    }
    ///All layer preparation and Canvas presentation shares one actual engine
    ///transaction. An error never leaves parent groups or motion updates live.
    pub fn publish_prepared(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        prepare_layers: Vec<crysirror_scene::layer::LayerCommand>,
        layers: Vec<crysirror_scene::layer::LayerCommand>,
    ) -> Result<()> {
        ensure!(
            client == self.client,
            "CatSystem2 foreign ImageList publication"
        );
        let batch = crysirror_scene::ServiceBatch {
            prepare_layers,
            canvases: self.commands()?,
            layers,
            ..Default::default()
        };
        host.submit_engine(client, 1, &serde_json::to_vec(&batch)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_key_order_clips_and_position_are_policy_over_shared_canvas() -> Result<()> {
        let mut list = ImageList::new(ClientId(1), 100, [8, 8])?;
        let image = "authored.png".to_owned();
        let draw = |sender, priority| Draw {
            key: ImageList::key(sender, priority).unwrap(),
            image: image.clone(),
            size: [2, 2],
            position: [3.9, -2.9],
            pivot: [1.9, -1.9],
            alpha: 128,
            visible: true,
            clip: None,
        };
        list.put(draw(8, 2))?;
        list.put(draw(7, 1))?;
        list.put(draw(6, 2))?;
        let key = ImageList::key(7, 1)?;
        assert!(list.clip(key, Some([1, 2, 5, 6])));
        list.put(draw(7, 1))?;
        let commands = list.commands()?;
        let CanvasCommand::Restore { surface, .. } = &commands[0] else {
            panic!()
        };
        let layers = surface
            .nodes
            .iter()
            .map(|node| {
                let CanvasNode::Draw { layer } = node else {
                    panic!()
                };
                layer
            })
            .collect::<Vec<_>>();
        assert_eq!(
            layers.iter().map(|l| l.id).collect::<Vec<_>>(),
            [key, ImageList::key(6, 2)?, ImageList::key(8, 2)?]
        );
        assert_eq!(layers[0].rect, [2., -1., 2., 2.]);
        assert_eq!(layers[0].clip, Some([-1., 3., 4., 4.]));
        assert!(list.clip(key, Some([5, 6, 1, 2])));
        assert!(list.records.get(&key).unwrap().1.clip.is_none());
        assert!(list.priority(ImageList::key(8, 2)?, -1));
        let commands = list.commands()?;
        let CanvasCommand::Restore { surface, .. } = &commands[0] else {
            panic!()
        };
        let CanvasNode::Draw { layer } = &surface.nodes[0] else {
            panic!()
        };
        assert_eq!(layer.id, ImageList::key(8, 2)?);
        assert!(list.remove(key));
        assert!(!list.remove(key));
        assert!(ImageList::key(65536, 1).is_err());
        Ok(())
    }
    #[cfg(feature = "runtime")]
    #[test]
    fn publication_uses_real_shared_images_canvas_views_and_existing_parent_motion() -> Result<()> {
        use crysirror_core::image_processing::{ImageInput, ImageProcessState};
        use crysirror_scene::layer::{LayerCommand as L, Paint};
        use std::{
            sync::Arc,
            time::{Duration, Instant},
        };
        let root = tempfile::tempdir()?;
        let project = serde_json::from_value(
            serde_json::json!({"schema":2,"name":"native ImageList ownership","content_revision":"1","assets":"game","canvas":[8,8],"clients":[{"name":"game","adapter":"crysirror-script-v1","game":"unused"}]}),
        )?;
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            root.path(),
            &root.path().join("state"),
            Default::default(),
        )?;
        let client = ClientId(1);
        let images = rt.host.image_metadata(client)?;
        let mut job = images.begin_process(
            ImageInput::Solid {
                size: [2, 2],
                color: [3, 7, 11, 255],
            },
            Arc::from(b"[]".as_slice()),
        )?;
        let start = Instant::now();
        let image = loop {
            match job.poll()? {
                ImageProcessState::Ready(image) => break image,
                ImageProcessState::Failed(error) => anyhow::bail!("{error}"),
                ImageProcessState::Pending => {
                    ensure!(
                        start.elapsed() < Duration::from_secs(5),
                        "shared image timeout"
                    );
                    std::thread::yield_now();
                }
            }
        };
        let receipt = rt.host.publish_image_resource(client, 610, image.clone())?;
        assert_eq!(image.rgba().as_ref(), [3, 7, 11, 255].repeat(4));
        let parent: LayerSpec = serde_json::from_value(
            serde_json::json!({"id":50,"rect":[0,0,2,2],"paint":{"kind":"solid","color":[0,0,0,0]},"opacity":128}),
        )?;
        rt.host.submit_engine(
            client,
            1,
            &serde_json::to_vec(&crysirror_scene::ServiceBatch {
                layers: vec![
                    L::Put { layer: parent },
                    L::BeginExternalMotion { id: 50, driver: 8 },
                ],
                ..Default::default()
            })?,
        )?;
        let motion = rt.host.engine.layers.motion_status(client, 50)?.drivers[0].clone();
        let mut list = ImageList::new(client, 100, [8, 8])?;
        list.put(Draw {
            key: ImageList::key(8, 1)?,
            image: receipt.path.relative().to_owned(),
            size: [2, 2],
            position: [2., 0.],
            pivot: [0.; 2],
            alpha: 255,
            visible: true,
            clip: None,
        })?;
        list.plane(Plane {
            canvas: 101,
            presentation: 51,
            parent: 50,
            source: [2, 0, 2, 2],
        })?;
        list.publish(&mut rt.host, client, vec![])?;
        let paint = rt.host.engine.layers.get(client, 51).unwrap().paint.clone();
        assert!(matches!(&paint, Paint::Snapshot { .. }));
        let frame = rt.host.engine.layers.frame([8, 8])?;
        let actual = frame
            .iter()
            .find(|p| matches!(p.paint, Paint::Snapshot { .. }))
            .unwrap();
        assert_eq!(actual.opacity, 128);
        assert_eq!(
            rt.host.engine.layers.motion_status(client, 50)?.drivers[0],
            motion
        );
        assert!(rt.host.engine.canvases.surface(ClientId(2), 100).is_none());
        assert!(list.publish(&mut rt.host, ClientId(2), vec![]).is_err());
        let before = serde_json::to_vec(rt.host.engine.canvases.surface(client, 100).unwrap())?;
        let mut proposed = list.clone();
        proposed.remove(ImageList::key(8, 1)?);
        proposed.plane(Plane {
            canvas: 102,
            presentation: 52,
            parent: 999,
            source: [0, 0, 2, 2],
        })?;
        assert!(proposed.publish(&mut rt.host, client, vec![]).is_err());
        assert_eq!(
            serde_json::to_vec(rt.host.engine.canvases.surface(client, 100).unwrap())?,
            before
        );
        assert!(rt.host.engine.layers.get(client, 52).is_none());
        //Rebuilding the current view does not mutate an old snapshot's Arc.
        list.remove(ImageList::key(8, 1)?);
        list.publish(&mut rt.host, client, vec![])?;
        let Paint::Snapshot { layers: old, .. } = paint else {
            unreachable!()
        };
        let Paint::Snapshot { layers: new, .. } =
            &rt.host.engine.layers.get(client, 51).unwrap().paint
        else {
            unreachable!()
        };
        assert!(!Arc::ptr_eq(&old, new));
        assert!(!old.is_empty());
        rt.host
            .release_image_resource(client, receipt.path.relative())?;
        Ok(())
    }
}
