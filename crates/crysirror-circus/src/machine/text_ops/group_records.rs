//!Typed native record identities survive append/trim/button migration.
//!Legacy unowned image ordinals are retained only for old logical snapshots.
use super::*;
impl Group {
    ///4187D0's fresh destination: associated owner0 is the message plane;
    ///owners1..N become controls0..N-1. Standalone source maps are not read.
    ///`variant == -1` selects by checkbox kind, otherwise by explicit ordinal.
    ///A None receipt is the original helper's return0, not a fabricated menu.
    pub(crate) fn native_message_group(&self,kind:i16,variant:i16)->Option<Self> {
        let mut group=Self::default();
        for source in self.buttons.iter().skip(1) {
            let mut button=Button{
                bounds:source.bounds,
                action:source.action.clone(),
                alternate_actions:source.alternate_actions.clone(),
                sounds:source.sounds.map(|sound|if sound==255{0}else{sound}),
                images:source.images.clone(),
                check_index:kind,
                check_value:source.check_value,
                ..Default::default()
            };
            //Only record160's checkbox policy is copied into the fresh
            //current record. Text/pivots/renderer effects and other flags reset.
            if source.behavior_flags.contains(&214){button.behavior_flags.insert(214);}
            group.buttons.push(button);
        }
        let selected=if variant == -1{kind}else{variant};
        let mut normal=false;let mut ordinal=0i32;
        for source in self.buttons.first().into_iter().flat_map(|button|&button.images) {
            let mut image=source.clone();
            if image.state==99&&!normal {
                normal=true;image.state=0;group.images.push(image);
            }else if image.state==2 {
                let take=ordinal==i32::from(selected);ordinal+=1;
                if take {image.state=0;group.images.push(image);}
            }
        }
        group.trim_record_metadata();
        (!group.images.is_empty()).then_some(group)
    }
    fn image_targets(&self)->Vec<UiRecordTarget> {
        if !self.image_order.is_empty(){return self.image_order.clone()}
        (0..self.images.len()).map(|index|UiRecordTarget::Image{button:None,index})
            .chain(self.buttons.iter().enumerate().flat_map(|(button,row)|(0..row.images.len()).map(move|index|UiRecordTarget::Image{button:Some(button),index}))).collect()
    }
    fn text_targets(&self)->Vec<UiRecordTarget> {
        if !self.text_order.is_empty(){return self.text_order.clone()}
        (0..self.texts.len()).map(|index|UiRecordTarget::Text{button:None,index})
            .chain(self.buttons.iter().enumerate().flat_map(|(button,row)|(0..row.texts.len()).map(move|index|UiRecordTarget::Text{button:Some(button),index}))).collect()
    }
    pub(crate) fn pushed_image(&mut self,target:UiRecordTarget) {
        if self.image_order.is_empty(){self.image_order=self.image_targets();}
        if self.image_order.last()!=Some(&target){self.image_order.push(target)}self.last_image=Some(target);
    }
    pub(crate) fn pushed_text(&mut self,target:UiRecordTarget) {
        if self.text_order.is_empty(){self.text_order=self.text_targets();}
        if self.text_order.last()!=Some(&target){self.text_order.push(target)}self.last_text=Some(target);
    }
    pub(crate) fn remap_record_targets(&mut self,mut map:impl FnMut(UiRecordTarget)->Option<UiRecordTarget>) {
        self.image_order=self.image_targets().into_iter().filter_map(&mut map).collect();
        self.text_order=self.text_targets().into_iter().filter_map(&mut map).collect();
        self.last_image=self.last_image.and_then(&mut map).or_else(||self.image_order.last().copied());
        self.last_text=self.last_text.and_then(&mut map).or_else(||self.text_order.last().copied());
        self.control_operations=self.control_operations.iter().filter_map(|operation|map(operation.target).map(|target|UiRecordOperation{target,attribute:operation.attribute.clone()})).collect();
        let mut animations=Vec::new();let mut owners=Vec::new();
        for (index,animation) in self.image_animations.iter().copied().enumerate() {
            let owner=self.image_animation_owners.get(index).copied().or_else(||self.image_animation_targets.get(index).copied().map(|index|UiRecordTarget::Image{button:None,index}));
            if let Some(target)=owner.and_then(&mut map){animations.push(animation);owners.push(target);}
        }
        self.image_animations=animations;
        self.image_animation_targets=owners.iter().map(|target|match target {UiRecordTarget::Image{button:None,index}=>*index,_=>0}).collect();
        self.image_animation_owners=owners;
    }
    pub(crate) fn trim_record_metadata(&mut self) {
        let images=self.images.len();let texts=self.texts.len();
        let buttons=self.buttons.iter().map(|button|(button.images.len(),button.texts.len())).collect::<Vec<_>>();
        self.remap_record_targets(|target|{
            let valid=match target {
                UiRecordTarget::Image{button:None,index}=>index<images,
                UiRecordTarget::Text{button:None,index}=>index<texts,
                UiRecordTarget::Image{button:Some(button),index}=>buttons.get(button).is_some_and(|counts|index<counts.0),
                UiRecordTarget::Text{button:Some(button),index}=>buttons.get(button).is_some_and(|counts|index<counts.1),
            };valid.then_some(target)
        });
    }
    pub(crate) fn retain_buttons(&mut self,mut retain:impl FnMut(usize,&Button)->bool) {
        self.image_order=self.image_targets();self.text_order=self.text_targets();
        let mut indices=vec![None;self.buttons.len()];let mut kept=Vec::new();
        for (old,button) in self.buttons.drain(..).enumerate(){if retain(old,&button){indices[old]=Some(kept.len());kept.push(button);}}
        self.buttons=kept;
        self.remap_record_targets(|target|match target {
            UiRecordTarget::Image{button:Some(button),index}=>indices.get(button).copied().flatten().map(|button|UiRecordTarget::Image{button:Some(button),index}),
            UiRecordTarget::Text{button:Some(button),index}=>indices.get(button).copied().flatten().map(|button|UiRecordTarget::Text{button:Some(button),index}),
            target=>Some(target),
        });
    }
    pub(crate) fn retain_images(&mut self,mut retain:impl FnMut(&mut ImagePart)->bool) {
        self.image_order=self.image_targets();self.text_order=self.text_targets();
        let mut indices=vec![None;self.images.len()];let mut kept=Vec::new();
        for (old,mut image) in self.images.drain(..).enumerate(){if retain(&mut image){indices[old]=Some(kept.len());kept.push(image);}}
        self.images=kept;
        self.remap_record_targets(|target|match target {UiRecordTarget::Image{button:None,index}=>indices.get(index).copied().flatten().map(|index|UiRecordTarget::Image{button:None,index}),target=>Some(target)});
    }
    ///418D90 copies672-byte button metadata, but rebuilds associated and
    ///standalone image/text renderers through418200/417B90/418460/417F10.
    ///Target group globals and its existing renderer records remain its own.
    pub(crate) fn append_records(&mut self,source:&Group) {
        let offset=[self.buttons.len(),self.images.len(),self.texts.len()];
        self.image_order=self.image_targets();self.text_order=self.text_targets();
        let map=|target|match target {
            UiRecordTarget::Image{button:None,index}=>UiRecordTarget::Image{button:None,index:index+offset[1]},
            UiRecordTarget::Text{button:None,index}=>UiRecordTarget::Text{button:None,index:index+offset[2]},
            UiRecordTarget::Image{button:Some(button),index}=>UiRecordTarget::Image{button:Some(button+offset[0]),index},
            UiRecordTarget::Text{button:Some(button),index}=>UiRecordTarget::Text{button:Some(button+offset[0]),index},
        };
        let associated=|target:&UiRecordTarget|matches!(target,UiRecordTarget::Image{button:Some(_),..}|UiRecordTarget::Text{button:Some(_),..});
        //418D90 iterates the associated tables before standalone tables.
        let images=source.image_targets();let texts=source.text_targets();
        let images=images.iter().copied().filter(&associated).chain(images.iter().copied().filter(|target|!associated(target))).map(map).collect::<Vec<_>>();
        let texts=texts.iter().copied().filter(&associated).chain(texts.iter().copied().filter(|target|!associated(target))).map(map).collect::<Vec<_>>();
        self.image_order.extend(images.iter().copied());self.text_order.extend(texts.iter().copied());
        let fresh_text=|part:&TextPart| {let mut part=part.clone();part.line_pitch=None;part.size=22;part};
        self.buttons.extend(source.buttons.iter().map(|button| {
            let mut button=button.clone();
            //Pivots belong to the newly constructed renderers, not Button672.
            button.image_pivots.clear();button.text_pivots.clear();
            button.texts=button.texts.iter().map(fresh_text).collect();button
        }));
        self.images.extend(source.images.iter().cloned().map(|mut image|{image.state=0;image}));
        self.texts.extend(source.texts.iter().map(fresh_text).map(|mut text|{text.state=0;text}));
        //Neither constructor copies pivot/effect/reveal/frame queues. Keep
        //target-owned attributes/animations; do not append those of source.
        self.last_image=images.last().copied().or(self.last_image);
        self.last_text=texts.last().copied().or(self.last_text);
    }
    ///Fresh4046C0 destination plus418D90(record copy), distinct from24/3's
    ///whole-object pointer transfer. Position/motion/group flags stay default.
    pub(crate) fn copy_native_records(&self)->Self {
        let mut group=Self::default();group.append_records(self);group
    }
}
