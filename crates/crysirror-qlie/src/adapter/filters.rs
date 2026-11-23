//! Translate native resource-filter syntax into the shared image worker graph.
//! Original immutable inputs stay leased so removing/replacing a filter is
//! reversible; repeated filter edits do not accumulate decoded resources.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageProcessState};
mod prepare;
pub(super) use prepare::Preparation;
use prepare::Step;
fn constant_mode(value: &str) -> Result<crysirror_image::rgb_lut::ConstantByteBlend> {
    use crysirror_image::rgb_lut::ConstantByteBlend as B;
    // Native557BD8 ->55E8EC ->538138. Numeric script IDs differ from
    // internal MMX kernel IDs; native names are accepted at the same boundary.
    let id = crate::expression::parse_integer(value).unwrap_or(-1);
    Ok(match (id, value.to_ascii_lowercase().as_str()) {
        (0 | 1, _) | (_, "" | "pass" | "normal") => B::Normal,
        (2, _) | (_, "screen") => B::Screen,
        (3, _) | (_, "multiple" | "multiply" | "modulate") => B::Multiply,
        (4, _) | (_, "difference") => B::Difference,
        (5, _) | (_, "lighten") => B::Lighten,
        (6, _) | (_, "darken") => B::Darken,
        (7, _) | (_, "add" | "lineardodge") => B::Add,
        (8, _) | (_, "sub") => B::Subtract,
        (9, _) | (_, "subnot" | "linearburn") => B::LinearBurn,
        (10, _) | (_, "exclusion") => B::Exclusion,
        (11, _) | (_, "overlay") => B::Overlay,
        (12, _) | (_, "hardlight") => B::HardLight,
        (13, _) | (_, "linearlight") => B::LinearLight,
        (14, _) | (_, "pinlight") => B::PinLight,
        _ => bail!("unsupported QLIE constant blend {value}"),
    })
}
fn luminance_map(a: &[String]) -> Result<serde_json::Value> {
    use crysirror_image::luma_lookup::{LookupIndex, LumaLookup};
    let n = |i: usize, default: i32| -> Result<i32> {
        Ok(a.get(i)
            .and_then(|v| crate::expression::parse_integer(v))
            .unwrap_or(default))
    };
    let color = |value: i32| {
        let c = value as u32;
        [(c >> 16) as u8, (c >> 8) as u8, c as u8, 0]
    };
    let level = |i, default| -> Result<u8> { Ok(n(i, default)?.clamp(0, 255) as u8) };
    let mut index = LookupIndex::WeightedRgb {
        coefficients: [77, 150, 29],
        fractional_bits: 8,
    };
    let stops = match a[0].as_str() {
        "binalize" => {
            let center = n(1, 128)?.clamp(0, 255);
            let power = ((f64::from(n(4, 100)?) / 100.) as f32).clamp(0., 1.);
            let width = (128. * f64::from(1. - power) + 0.5).floor() as i32;
            vec![
                ((center - width).clamp(0, 255) as u8, color(n(2, 0)?)),
                ((center + width).clamp(0, 255) as u8, color(n(3, 0xffffff)?)),
            ]
        }
        "gmap2" => {
            if a.get(5).is_some_and(|v| v == "alpha") {
                index = LookupIndex::Alpha;
            }
            vec![
                (level(3, 0)?, color(n(1, 0)?)),
                (level(4, 255)?, color(n(2, 0xffffff)?)),
            ]
        }
        "gmap3" => {
            if a.get(7).is_some_and(|v| v == "alpha") {
                index = LookupIndex::Alpha;
            }
            let expanded =
                a.get(5).is_some_and(|v| !v.is_empty()) || a.get(6).is_some_and(|v| !v.is_empty());
            let levels = if expanded {
                [level(4, 0)?, level(5, 127)?, level(6, 255)?]
            } else {
                [0, level(4, 127)?, 255]
            };
            vec![
                (levels[0], color(n(1, 0)?)),
                (levels[1], color(n(2, 0x888888)?)),
                (levels[2], color(n(3, 0xffffff)?)),
            ]
        }
        _ => bail!("QLIE unknown colour-map method"),
    };
    let spec = LumaLookup::gradient_byte1024(&stops, index, true)?;
    Ok(json!({"kind":"luma_lookup","spec":spec}))
}
impl QlieVm {
    pub(super) fn image_filter(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        index: usize,
        source: &str,
    ) -> Result<bool> {
        // 5D23C0 records the filter text in the object's 32 slot strings
        // (+408) and then refilters the images that exist (5D23F0). A slot
        // without an image keeps only the recorded text.
        ensure!(index < 32, "QLIE filter slot");
        self.objects
            .get_mut(id)
            .unwrap()
            .properties
            .insert(format!("fo/{index}"), vec![Value::Text(source.to_owned())]);
        let o = &self.objects[id];
        let slot = if let Some(bundle) = &o.bundle {
            bundle.images.get(index).map(|v| v.slot)
        } else if index == 0 {
            Some(0)
        } else {
            None
        };
        let Some(slot) = slot.filter(|i| o.images.get(*i).is_some_and(Option::is_some)) else {
            return Ok(true);
        };
        if source.is_empty() {
            if let Some(original) = self.objects.get_mut(id).unwrap().originals.remove(&slot) {
                let old = self.objects.get_mut(id).unwrap().images[slot].replace(original);
                if let Some(old) = old {
                    h.release_image_resource(self.client, old.path.relative())?;
                }
            }
            return Ok(true);
        }
        if self.filter_job.is_none() {
            let steps = self.filter_steps(source)?;
            let o = self.objects.get_mut(id).unwrap();
            let original = o
                .originals
                .entry(slot)
                .or_insert_with(|| o.images[slot].as_ref().unwrap().clone());
            let resource = h
                .published_image_resource(self.client, original.path.relative())?
                .context("QLIE immutable filter input lease")?;
            self.filter_job = Some(Preparation::new(id, slot, source, steps, resource));
        }
        let job = self.filter_job.as_mut().unwrap();
        ensure!(
            job.matches(id, slot, source),
            "QLIE filter continuation identity"
        );
        let Some(image) = job.poll(h, self.client, &self.pool)? else {
            return Ok(false);
        };
        self.filter_job = None;
        self.next_image = self
            .next_image
            .checked_add(1)
            .context("QLIE filter resource id")?;
        let published = h.publish_image_resource(self.client, self.next_image, image)?;
        let o = self.objects.get_mut(id).unwrap();
        let old = o.images[slot].replace(published).unwrap();
        if old.path != o.originals[&slot].path {
            h.release_image_resource(self.client, old.path.relative())?;
        }
        Ok(true)
    }
    fn filter_steps(&self, source: &str) -> Result<Vec<Step>> {
        let mut steps = Vec::new();
        let mut effects = Vec::new();
        let mut commands = 0usize;
        for command in crate::program::split_native(source, "\\", false, false) {
            if command.trim().is_empty() {
                continue;
            }
            commands += 1;
            ensure!(commands <= 16, "QLIE filter command limit");
            let a = crate::program::arguments(&command)?;
            if a.is_empty() {
                continue;
            }
            let n = |i: usize, default: i32| -> Result<i32> {
                Ok(a.get(i)
                    .and_then(|v| crate::expression::parse_integer(v))
                    .unwrap_or(default))
            };
            match a[0].as_str() {
                "blend"=>{
                    if !effects.is_empty() {
                        steps.push(Step::Effects(serde_json::to_vec(&std::mem::take(&mut effects))?.into()));
                    }
                    //559D48 ->55DBD4: arg10 is a root-relative filename;
                    //arg11==back replaces the base before drawing the old image.
                    let raw=a.get(10).map(String::as_str).unwrap_or("");
                    let raw=raw.strip_prefix('"').and_then(|v|v.strip_suffix('"')).unwrap_or(raw);
                    let key=crate::program::canonical(raw)?;
                    let path=self.config.assets.get(&key).with_context(||format!("QLIE blend resource absent: {raw}"))?.clone();
                    let tint=n(6,0xffffff)? as u32;
                    let add=n(7,0)? as u32;
                    let blend=constant_mode(a.get(9).map(String::as_str).unwrap_or("1"))?;
                    let spec=crysirror_image::affine_blend::AffineBlendByte128{
                        position:[n(1,0)?,n(2,0)?],scale:[(f64::from(n(3,100)?)/100.) as f32,(f64::from(n(4,100)?)/100.) as f32],
                        //561634 consumes turns directly, with pi=3.1415.
                        //Retain the native default100 and both half-pixel shifts.
                        rotation_turns:n(5,100)? as f32,angle_pi:3.1415,
                        center_offset:[-0.5,-0.5],odd_source_offset:[-0.5,-0.5],
                        multiply:[(tint>>16) as u8,(tint>>8) as u8,tint as u8,n(8,255)? as u8],
                        add:[(add>>16) as u8,(add>>8) as u8,add as u8],blend,
                        //55FC40/53B40C: four optional half-size levels above16.
                        //The inspected MMX kernel repeats the low horizontal
                        //sample and the last output row's first source row.
                        mip:Some(crysirror_image::affine_blend::ByteMipProfile{levels:4,min_extent:16,average_x:false,repeat_last_row:true}),
                    };
                    spec.validate()?;
                    steps.push(Step::Blend{path,spec,back:a.get(11).is_some_and(|v|v=="back")});
                }
                "goverlay"=>{
                    let color=|value:i32,alpha:i32|{let c=value as u32;[(c>>16) as u8,(c>>8) as u8,c as u8,alpha as u8]};
                    let colors=[color(n(3,0)?,n(2,255)?),color(n(5,0)?,n(4,0)?),color(n(7,0)?,n(6,0)?)];
                    let blend=constant_mode(a.get(11).map(String::as_str).unwrap_or("1"))?;
                    effects.push(json!({"kind":"gradient_overlay","spec":{"vertical":a.get(1).is_some_and(|v|v=="vertical"),"positions_percent":[n(8,0)?,n(9,100)?,n(10,100)?],"colors":colors,"blend":blend,"opacity":n(12,255)? as u8}}));
                }
                "blurmask"=>{
                    let x=n(11,0)?;let y=n(12,0)?;
                    ensure!((0..=16384).contains(&x)&&(0..=16384).contains(&y),"QLIE blurmask window bounds");
                    let color=n(13,0xffffff)? as u32;
                    let color_mode=constant_mode(a.get(15).map(String::as_str).unwrap_or("1"))?;
                    let blend=constant_mode(a.get(16).map(String::as_str).unwrap_or("1"))?;
                    effects.push(json!({"kind":"masked_blur","spec":{
                        "blur":{"radii":[x/2,y/2],"passes":2},
                        "color":[(color>>16) as u8,(color>>8) as u8,color as u8,n(14,255)? as u8],"color_mode":color_mode,
                        "mask":{"translation":[n(1,0)?,n(2,0)?],"scale_percent":[n(3,100)?.max(1),n(4,100)?.max(1)],"rotation_turns":(-f64::from(n(5,0)?)/360.) as f32,"feather":n(6,0)?.max(1),"contrast":(f64::from(n(7,0)?.clamp(0,100))/100.) as f32,"shape":if a.get(10).is_some_and(|s|s=="circle") {"circle"}else{"rectangle"},"invert":n(9,0)?!=1,"amount256":256,"circle_center_coverage":0.},
                        "blend":blend,"opacity":n(8,255)? as u8
                    }}));
                }
                "distwave"=>{
                    use crysirror_image::{byte_sampling::ByteSampleEdge as E,wave_displace::{WaveShape as W,WaveDisplaceByte128}};
                    let wave=match a.get(1).map(String::as_str).unwrap_or("sin") {
                        ""|"sin"=>W::Sin,"abssin"=>W::AbsSin,"square"=>W::Square,"abssquare"=>W::AbsSquare,
                        "triangle"=>W::Triangle,"abstriangle"=>W::AbsTriangle,"saw"=>W::Saw,"abssaw"=>W::AbsSaw,
                        "random"=>W::Random,"absrandom"=>W::AbsRandom,"randomlinear"=>W::RandomLinear,"absrandomlinear"=>W::AbsRandomLinear,
                        value=>bail!("unsupported QLIE distwave waveform {value}"),
                    };
                    let edge=match a.get(2).map(String::as_str).unwrap_or("clip") {
                        ""|"clip"=>E::ClampTransparent,"wrap"=>E::SignMaskedWrap,"border"=>E::Clamp,"mirror"=>E::AbsoluteMirror,
                        value=>bail!("unsupported QLIE distwave edge {value}"),
                    };
                    let phase=|degrees:i32|(f64::from(degrees.wrapping_add(360000)%360))/360.;
                    let noise=if wave.uses_noise() {self.random.as_ref().context("QLIE wave noise bank not prepared")?.noise_words()}else{Vec::new()};
                    let spec=WaveDisplaceByte128{wave,edge,periods:[n(3,100)?.max(1) as u32,n(6,100)?.max(1) as u32],amplitudes:[n(4,0)? as f32,n(7,0)? as f32],phases:[phase(n(5,0)?),phase(n(8,0)?)],parallel:a.get(9).is_some_and(|v|v=="parallel"),noise};
                    effects.push(json!({"kind":"wave_displace","spec":spec}));
                }
                "resize"=>{
                    //558A3C ->53B1B8: preserve the input when either requested
                    //axis is below2. Omitted dimensions follow earlier filters.
                    let axis=|i:usize|a.get(i).and_then(|v|crate::expression::parse_integer(v));
                    let requested=[axis(1),axis(2)];
                    if !requested.iter().flatten().any(|v|*v<2) {
                        let size=requested.map(|v|v.map(|v|v as u32));
                        let sampling=if a.get(3).is_some_and(|v|v=="point"||v=="0") {"point"}else{"linear"};
                        effects.push(json!({"kind":"resize_byte","spec":{"size":size,"sampling":sampling}}));
                    }
                }
                "clipmask"=>{
                    //55F0D8 clamps host parameters and converts its byte
                    //amount to a full256-step alpha multiplier before55EE88.
                    let amount=((n(8,255)?.clamp(0,255) as u32)<<8)/255;
                    let circle=a.get(10).is_some_and(|s|s=="circle");
                    effects.push(json!({"kind":"shape_mask","spec":{"translation":[n(1,0)?,n(2,0)?],"scale_percent":[n(3,100)?.max(1),n(4,100)?.max(1)],"rotation_turns":(-f64::from(n(5,0)?)/360.) as f32,"feather":n(6,0)?.max(1),"contrast":(f64::from(n(7,0)?.clamp(0,100))/100.) as f32,"shape":if circle {"circle"}else{"rectangle"},"invert":n(9,0)?==1,"amount256":amount,"circle_center_coverage":0.}}));
                }
                "zoom"|"zoom2"=>{
                    let x=n(3,100)?;
                    let scale=[(f64::from(x)/100.) as f32,(f64::from(n(4,x)?)/100.) as f32];
                    let geometry=if a[0]=="zoom" {json!({"kind":"centered_destination","center":[n(1,0)?,n(2,0)?],"scale":scale})}
                        else {json!({"kind":"source_pivot","pivot":[n(1,0)?,n(2,0)?],"scale":scale})};
                    effects.push(json!({"kind":"rect_resample","spec":{"geometry":geometry,"retained_border":[1,1,2,2]}}));
                }
                "stretch"=>{
                    let bounds=[n(1,0)?,n(2,0)?,n(3,0)?,n(4,0)?];
                    if bounds[0]<bounds[2]&&bounds[1]<bounds[3] {
                        effects.push(json!({"kind":"rect_resample","spec":{"geometry":{"kind":"rectangle","bounds":bounds},"retained_border":[1,1,2,2]}}));
                    }
                }
                "sphere"=>effects.push(json!({"kind":"cosine_lens","spec":{"amount":((f64::from(n(1,0)?)/100.) as f32).clamp(-1.,1.),"axes":[n(2,0)?==1,n(3,0)?==1]}})),
                "binalize"|"gmap2"|"gmap3"=>effects.push(luminance_map(&a)?),
                "gamma" => {
                    let spec=crysirror_image::rgb_lut::RgbLookup::gamma_byte_ramp((f64::from(n(1,100)?)/100.) as f32)?;
                    effects.push(json!({"kind":"rgb_lookup","spec":spec}));
                }
                "bright" => {
                    let spec=crysirror_image::rgb_lut::RgbLookup::brightness_contrast_byte250((f64::from(n(1,0)?)/100.) as f32,(f64::from(n(2,0)?)/100.) as f32)?;
                    effects.push(json!({"kind":"rgb_lookup","spec":spec}));
                }
                "range" | "toneline" => {
                    let (input,output)=if a[0]=="range" {
                        let (lo,hi)=(n(1,0)?,n(2,0)?);
                        ([lo.min(hi),lo.max(hi)],[0,255])
                    } else { ([n(1,0)?,n(3,255)?],[n(2,0)?,n(4,255)?]) };
                    //55F344 returns before applying its temporary table in
                    //the equal-input/equal-output branches. Retain that native
                    //policy; the shared ramp itself defines both degeneracies.
                    if input[0]!=input[1] && output[0]!=output[1] {
                        let spec=crysirror_image::rgb_lut::RgbLookup::linear_byte_ramp(input,output)?;
                        effects.push(json!({"kind":"rgb_lookup","spec":spec}));
                    }
                }
                "color" => {
                    use crysirror_image::rgb_lut::ConstantByteBlend as B;
                    //5588A0's six values are a separate ABI from colorfill's
                    //fourteen named modes (55E8EC internal IDs3..8).
                    let mode=match n(2,0)? {
                        0=>Some(B::Normal),1=>Some(B::Screen),2=>Some(B::Multiply),
                        3=>Some(B::Difference),4=>Some(B::Lighten),5=>Some(B::Darken),
                        _=>None, //Original switch emits no operation.
                    };
                    if let Some(mode)=mode {
                        let color=n(1,0)? as u32;
                        let spec=crysirror_image::rgb_lut::RgbLookup::constant_byte256([(color>>16) as u8,(color>>8) as u8,color as u8,(color>>24) as u8],mode);
                        effects.push(json!({"kind":"rgb_lookup","spec":spec}));
                    }
                }
                "chroma" => {
                    //55E98C is an RGB mixing matrix, not HSL saturation. Its
                    //neutral factor also retains the original255/256 product.
                    let amount=((f64::from(n(1,0)?)/100.) as f32).clamp(-1.,10.);
                    let side=((-f64::from(amount)*85.) as f32).round() as i32;
                    let main=255-2*side;
                    effects.push(json!({"kind":"integer_color_matrix","spec":{"matrix":[[main,side,side],[side,main,side],[side,side,main]],"offset":[0,0,0],"divisor":256}}));
                }
                "huechroma" => {
                    let quant=|v:i32,divisor:f64,lo:f32,hi:f32,scale:f64| {
                        ((f64::from(((f64::from(v)/divisor) as f32).clamp(lo,hi))*scale) as f32).round() as i32
                    };
                    effects.push(json!({"kind":"integer_color_modulation","spec":{"hue":quant(n(1,0)?,180.,-1.,1.,768.),"replace_hue":n(4,0)?==1,"saturation256":quant(n(2,0)?,100.,-1.,10.,256.),"luminance256":quant(n(3,0)?,100.,-1.,1.,256.)}}));
                }
                "blur" => {
                    let x = n(1, 4)?;
                    let y = n(2, x)?;
                    ensure!(
                        (0..=16384).contains(&x) && (0..=16384).contains(&y),
                        "QLIE blur radius"
                    );
                    //560A54 ->5572BC runs two complete clamped2D passes;
                    //arguments are window diameters rounded down to radii.
                    effects.push(json!({"kind":"box_blur_clamped","spec":{"radii":[x/2,y/2],"passes":2}}));
                }
                "colorfill" => {
                    let mut color = n(1, 0)? as u32;
                    let mode_index=if a.len()>=4 {
                        color|=(n(2,255)? as u8 as u32)<<24;
                        3
                    } else { 2 };
                    let mode = constant_mode(a.get(mode_index).map(String::as_str).unwrap_or("1"))?;
                    let spec = crysirror_image::rgb_lut::RgbLookup::constant_byte256(
                        [
                            (color >> 16) as u8,
                            (color >> 8) as u8,
                            color as u8,
                            (color >> 24) as u8,
                        ],
                        mode,
                    );
                    effects.push(json!({"kind":"rgb_lookup","spec":spec}));
                }
                "flip" => effects
                    .push(json!({"kind":"flip","horizontal":n(1,0)?!=0,"vertical":n(2,0)?!=0})),
                "mirror" => effects
                    .push(json!({"kind":"flip","horizontal":n(1,0)?==1,"vertical":n(2,0)?==1})),
                "mosaic" | "mosic" => {
                    //55CBB4 averages all four stored bytes independently. It
                    //does nothing unless both cell axes are at least two.
                    let x=n(1,8)?;
                    let y=n(2,x)?;
                    if x>=2 && y>=2 { effects.push(json!({"kind":"block_average","cell":[x.min(1024),y.min(1024)]})); }
                }
                "canvas" => effects.push(json!({"kind":"canvas_margins","margins":[n(1,0)?,n(2,0)?,n(3,0)?,n(4,0)?],"fill":[136,136,136,0]})),
                _ => bail!("unsupported QLIE resource filter {command}"),
            }
        }
        if !effects.is_empty() {
            steps.push(Step::Effects(serde_json::to_vec(&effects)?.into()));
        }
        Ok(steps)
    }
}
