//! Original current-button records (412A30/418200/419710/414180).
//! Shared Ui owns focus, hit testing and pointer capture; these records retain
//! only the native scalar binding, state and image/text anchor policy.
use super::*;

impl NativeSlider {
    pub fn target(&self)->(i16,i16) {((self.variable/1000) as i16,(self.variable%1000) as i16)}
    pub fn step(&self)->f32 {((f64::from(self.maximum)-f64::from(self.minimum))/f64::from(i32::from(self.stops)-1)) as f32}
    pub fn pixel(&self,value:i32)->i32 {
        native_ftol(f64::from(value.min(i32::from(self.stops)-1))*f64::from(self.step())+f64::from(self.minimum))
    }
    pub fn pointer_value(&self,x:i32)->i32 {
        if x<=i32::from(self.minimum){0}
        else if x>=i32::from(self.maximum){i32::from(self.stops)-1}
        else{native_ftol(f64::from(x-i32::from(self.minimum))/f64::from(self.step()))}
    }
}
/// Modern pinned EXE45B550 cvttsd2si path, including indefinite integer.
pub(crate) fn native_ftol(value:f64)->i32 {
    if value.is_finite()&&value>=f64::from(i32::MIN)&&value<2147483648. {value.trunc() as i32}else{i32::MIN}
}
///414BC0 image state rules;103/105 are persistent slider paints.
pub(crate) fn image_visible(mode:i16,state:i16)->bool {
    match mode {100=>state==99,101|102|104=>state!=99,3=>state==2,100.. =>true,_=>mode==state}
}
impl Machine {
    pub(super) fn declare_button_image(&mut self,a:[i16;8],host:&mut dyn NativeHost)->Result<bool> {
        if a[7]==5 {
            let button=self.state.current_button.as_mut().context("Circus slider without current button")?;
            button.slider=Some(NativeSlider{minimum:a[0],maximum:a[1],stops:a[2],variable:i32::from(a[4])+1000*i32::from(a[3]),initial_value:0});
            return Ok(true)
        }
        if a[7]==12 {
            return self.set_control_attribute(host,a[2]!=0,UiRecordAttribute::Pivot([a[0],a[1]]))
        }
        let mut part=image_part(a)?;
        part.bind_resource(host)?;
        //418200/419710 append no record for a null native texture pointer.
        if host.image_dimensions(part.slot)?.is_none(){return Ok(true)}
        part.state=match a[7]{6=>100,8=>101,10=>102,11=>3,value=>value};
        //4196D0 explicitly supplies flag0; only418200's default dispatch
        //takes the current1B flag. No high bit of the resource slot is used.
        part.auxiliary_flag=!matches!(a[7],6|8|10|11)&&self.state.visual.frame_toggle;
        let button=self.state.current_button.as_mut().context("Circus state image without button")?;
        ensure!(button.images.len()<128,"Circus per-button image limit");
        button.images.push(part);
        let target=UiRecordTarget::Image{button:Some(self.state.builder.buttons.len()),index:button.images.len()-1};
        self.state.builder.pushed_image(target);
        Ok(true)
    }
    pub(super) fn set_control_attribute(&mut self,host:&mut dyn NativeHost,associated:bool,attribute:UiRecordAttribute)->Result<bool> {
        //414180/4142A0 choose between image collections80 and144. Neither
        //selector addresses the separate text maps112/176.
        let target=if associated {
            self.state.builder.image_order.iter().rev().copied().find(|target|matches!(target,UiRecordTarget::Image{button:Some(_),..}))
        }else{self.state.builder.images.len().checked_sub(1).map(|index|UiRecordTarget::Image{button:None,index})};
        let Some(target)=target else{return Ok(true)};
        let operation=UiRecordOperation{target,attribute};
        if !host.control_attribute(self.state.selected_builder(),&operation)?{return Ok(false)}
        let category=operation.attribute.category();
        self.state.builder.control_operations.retain(|previous|previous.target!=target||previous.attribute.category()!=category);
        self.state.builder.control_operations.push(operation);Ok(true)
    }
    pub(crate) fn sync_menu_variables(&mut self,host:&mut dyn NativeHost)->Result<()> {
        for change in host.take_menu_values(){self.state.assign(change.bank,change.index,change.value)?;}
        let mut current=Vec::new();
        for (bank,index) in host.menu_bindings(){current.push(MenuValue{bank,index,value:self.value(host,bank,index)?});}
        if !current.is_empty(){host.sync_menu_values(&current)?;}
        Ok(())
    }
    pub(crate) fn resolve_group_values(&self,host:&mut dyn NativeHost,group:&mut Group)->Result<()> {
        for button in &mut group.buttons {
            if let Some(slider)=&mut button.slider {let (bank,index)=slider.target();slider.initial_value=self.value(host,bank,index)?;}
            if button.behavior_flags.contains(&214){button.check_value=self.value(host,93,button.check_index)?;}
        }
        Ok(())
    }
}

impl ImagePart {
    pub(crate) fn bind_resource(&mut self,host:&mut dyn NativeHost)->Result<()> {
        self.resource_binding=host.image_identity(self.slot)?;self.binding_declared=true;Ok(())
    }
}
