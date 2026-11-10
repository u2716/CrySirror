struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) filter_index: u32,
    @location(3) @interpolate(flat) sample_bounds: vec4<f32>,
    @location(4) @interpolate(flat) clip_rect: vec4<f32>,
    // GLSL ES has no portable noperspective qualifier. Perspective-interpolate
    // (screen_position * clip_w, clip_w), then divide in the fragment stage.
    @location(5) clip_position: vec3<f32>,
}
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
fn vertex(position: vec4<f32>, uv: vec2<f32>, color: vec4<f32>, filter_slot: f32) -> Out {
    var out: Out;
    out.position = position;
    out.uv = uv;
    out.color = color;
    out.filter_index = u32(max(filter_slot, 0.0));
    out.sample_bounds = vec4<f32>(-1e20, -1e20, 1e20, 1e20);
    out.clip_rect = out.sample_bounds;
    out.clip_position = vec3<f32>(position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5 * position.w), position.w);
    return out;
}
@vertex fn vs_main(@location(0) position: vec4<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>, @location(3) filter_slot: f32, @location(4) sample_bounds: vec4<f32>, @location(5) clip_rect: vec4<f32>) -> Out {
    var out = vertex(position, uv, color, filter_slot);
    out.sample_bounds = sample_bounds;
    out.clip_rect = clip_rect;
    return out;
}
@vertex fn vs_plain(@location(0) position: vec4<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>, @location(3) filter_slot: f32) -> Out {
    return vertex(position, uv, color, filter_slot);
}
fn outside_clip(in: Out) -> bool {
    // The common perspective divisor cancels, recovering screen-linear UV
    // even for projective quads with different clip-space w at each vertex.
    let screen_position = in.clip_position.xy / in.clip_position.z;
    return any(screen_position < in.clip_rect.xy) || any(screen_position >= in.clip_rect.zw);
}
fn bounded_uv(uv: vec2<f32>, bounds: vec4<f32>) -> vec2<f32> {
    let half_texel = vec2<f32>(0.5) / vec2<f32>(textureDimensions(image, 0));
    let lo = bounds.xy + half_texel;
    let hi = bounds.zw - half_texel;
    let middle = (lo + hi) * 0.5;
    return clamp(uv, min(lo, middle), max(hi, middle));
}
@fragment fn fs_main(in: Out) -> @location(0) vec4<f32> {
    if outside_clip(in) { discard; }
    return textureSampleLevel(image, image_sampler, bounded_uv(in.uv, in.sample_bounds), 0.0) * in.color;
}

@fragment fn fs_premultiply(in: Out) -> @location(0) vec4<f32> {
    if outside_clip(in) { discard; }
    let c = textureSampleLevel(image, image_sampler, bounded_uv(in.uv, in.sample_bounds), 0.0) * in.color;
    return vec4<f32>(c.rgb * c.a, c.a);
}

// ---- Layer filters (ADR 0021): the Siglus/tona3 sprite effect stages. ----
struct LayerFilter {
    // mono, invert, bright, dark
    a: vec4<f32>,
    // color rate, target rgb
    b: vec4<f32>,
    // add rgb, flags (1 tone curve, 2 mask, 4 gray alpha, 8 alpha test)
    c: vec4<f32>,
    // curve row, curve saturation, distortion kind (0 none, 1 mosaic, 2 raster h,
    // 3 raster v, 4 radial blur, 5 threshold fade, 6 inverted threshold, 7 wave h, 8 wave v), p0
    d: vec4<f32>,
    // p1, p2, p3, p4
    e: vec4<f32>,
    // RGB luminance coefficients, sampled-alpha exponent.
    f: vec4<f32>,
    // Selected encoded mask byte (0 RGBA multiply), signed byte offset.
    g: vec4<f32>,
    h: vec4<f32>,
    // Stored blend: mode+1, destination origin XY, source/output alpha bits.
    i: vec4<f32>,
};
@group(1) @binding(0) var<storage, read> filters: array<LayerFilter>;
@group(1) @binding(1) var mask_image: texture_2d<f32>;
@group(1) @binding(2) var curve_image: texture_2d<f32>;
@group(1) @binding(3) var destination_image: texture_2d<f32>;
@group(2) @binding(0) var ordered_coverage: texture_2d<f32>;

fn to_gamma(c: vec3<f32>) -> vec3<f32> {
    let x = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    return select(1.055 * pow(x, vec3<f32>(1.0 / 2.4)) - 0.055, x * 12.92, x <= vec3<f32>(0.0031308));
}
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let x = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    return select(pow((x + 0.055) / 1.055, vec3<f32>(2.4)), x / 12.92, x <= vec3<f32>(0.04045));
}
// Out-of-texture taps are transparent (Siglus sample_tex0_safe).
fn encoded_load(pixel:vec2<i32>,size:vec2<i32>)->vec4<f32> {
    let p=((pixel%size)+size)%size;
    let c=textureLoad(image,p,0);
    return vec4<f32>(to_gamma(c.rgb),c.a);
}
fn sample_axis(pixel:i32,size:i32,repeat:bool,mirror:bool)->i32 {
    if mirror {
        let period=2*size;let p=((pixel%period)+period)%period;
        return select(p,period-1-p,p>=size);
    }
    if repeat {return ((pixel%size)+size)%size;}
    return clamp(pixel,0,size-1);
}
fn encoded_byte_load(pixel:vec2<i32>,size:vec2<i32>,flags:u32)->vec4<i32> {
    let p=vec2<i32>(sample_axis(pixel.x,size.x,(flags&256u)!=0u,(flags&512u)!=0u),
        sample_axis(pixel.y,size.y,(flags&1024u)!=0u,(flags&2048u)!=0u));
    let c=textureLoad(image,p,0);
    return vec4<i32>(round(vec4<f32>(to_gamma(c.rgb),c.a)*255.0));
}
fn sample_color(uv:vec2<f32>,f:LayerFilter)->vec4<f32> {
    let flags=u32(f.c.w);
    if (flags&128u)!=0u {
        let size=vec2<i32>(textureDimensions(image,0));
        if (flags&64u)!=0u {
            return vec4<f32>(encoded_byte_load(vec2<i32>(floor(uv*vec2<f32>(size))),size,flags))/255.0;
        }
        let p=uv*vec2<f32>(size)-vec2<f32>(0.5);
        let base=vec2<i32>(floor(p));
        let fraction=vec2<u32>(floor(fract(p)*65536.0));
        let weight=(fraction+vec2<u32>(1u))>>vec2<u32>(9u);
        let a=encoded_byte_load(base,size,flags);
        let b=encoded_byte_load(base+vec2<i32>(1,0),size,flags);
        let c=encoded_byte_load(base+vec2<i32>(0,1),size,flags);
        let d=encoded_byte_load(base+vec2<i32>(1,1),size,flags);
        let top=a+(((b-a)*i32(weight.x))>>vec4<u32>(7u));
        let bottom=c+(((d-c)*i32(weight.x))>>vec4<u32>(7u));
        return vec4<f32>(top+(((bottom-top)*i32(weight.y))>>vec4<u32>(7u)))/255.0;
    }
    if (u32(f.c.w)&32u)!=0u {
        let size=vec2<i32>(textureDimensions(image,0));
        if (u32(f.c.w)&64u)!=0u {
            return encoded_load(vec2<i32>(floor(uv*vec2<f32>(size))),size);
        }
        let p=uv*vec2<f32>(size)-vec2<f32>(0.5);
        let base=vec2<i32>(floor(p));let weight=fract(p);
        return mix(mix(encoded_load(base,size),encoded_load(base+vec2<i32>(1,0),size),weight.x),
                   mix(encoded_load(base+vec2<i32>(0,1),size),encoded_load(base+vec2<i32>(1,1),size),weight.x),weight.y);
    }
    let c=textureSampleLevel(image,image_sampler,uv,0.0);
    return vec4<f32>(to_gamma(c.rgb),c.a);
}
fn tap(uv: vec2<f32>, f:LayerFilter) -> vec4<f32> {
    let c = sample_color(uv,f);
    let inside = (u32(f.c.w)&32u)!=0u || (all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)));
    return select(vec4<f32>(0.0), c, inside);
}
fn tap_clamped(uv: vec2<f32>,f:LayerFilter) -> vec4<f32> {
    let half_texel=vec2<f32>(0.5)/vec2<f32>(textureDimensions(image,0));
    return sample_color(clamp(uv,half_texel,vec2<f32>(1.0)-half_texel),f);
}
fn raster_amp(progress: f32) -> f32 {
    let rp = clamp(1.0 - progress, 1e-4, 1.0);
    let lv = max((1.0 - rp) * 100.0, 1e-4);
    return 1.0 - ((log(lv) / log(10.0)) + 1.0) / 3.0;
}
fn distorted(uv: vec2<f32>, f: LayerFilter) -> vec4<f32> {
    let kind = f.d.z;
    let p0 = f.d.w;
    let p1 = f.e.x;
    let p2 = f.e.y;
    let p3 = f.e.z;
    if kind > 8.5 && kind < 9.5 {
        var c=tap_clamped(uv,f)*p2;
        for(var y:i32=-1;y<=1;y=y+1){for(var x:i32=-1;x<=1;x=x+1){
            if x!=0 || y!=0 {c=c+tap_clamped(uv+vec2<f32>(f32(x)*p0,f32(y)*p1),f)*p3;}
        }}
        return c;
    }
    if kind > 9.5 && kind < 10.5 {
        return tap(floor(uv*vec2<f32>(p0,p1)+vec2<f32>(p2,p3))/vec2<f32>(p0,p1),f);
    }
    if kind > 0.5 && kind < 1.5 {
        let cu = max(p0, 1e-5);
        let cv = max(p0 * max(p1, 1e-5), 1e-5);
        var c = tap(vec2<f32>(floor(uv.x / cu) * cu, floor(uv.y / cv) * cv),f);
        c.a = 1.0;
        return c;
    }
    if kind > 1.5 && kind < 3.5 {
        let vertical = kind > 2.5;
        let fnn = max(p0, 1.0);
        var t = fract(select(uv.y, uv.x, vertical) * fnn);
        t = (t - fnn * 0.1) / fnn;
        let o = sin(3.14159265 * p3 * p2 + t * 3.14159265 * p1) * raster_amp(p3);
        return tap(select(vec2<f32>(uv.x + o, uv.y), vec2<f32>(uv.x, uv.y + o), vertical),f);
    }
    if kind > 3.5 && kind < 4.5 {
        let dims = vec2<f32>(textureDimensions(image, 0));
        let texel = 1.0 / max(max(dims.x, dims.y), 1.0);
        var dir = vec2<f32>(p0, p1) - uv;
        let len = length(dir);
        if len <= 1e-5 || p2 <= 1e-5 {
            return tap(uv,f);
        }
        dir = normalize(dir) * texel * p2 * len * max(p3, 0.0);
        var w = array<f32, 10>(0.19, 0.17, 0.15, 0.13, 0.11, 0.09, 0.07, 0.05, 0.03, 0.01);
        var c = vec4<f32>(0.0);
        for (var i = 0; i < 10; i = i + 1) {
            c = c + tap(uv + dir * f32(i),f) * w[i];
        }
        c.a = 1.0;
        return c;
    }
    if kind > 6.5 {
        let vertical = kind > 7.5;
        let o = p0 * sin(6.28318531 * (select(uv.y, uv.x, vertical) * p1 + p2));
        return tap(select(vec2<f32>(uv.x + o, uv.y), vec2<f32>(uv.x, uv.y + o), vertical),f);
    }
    var c = tap(uv,f);
    if kind > 4.5 {
        let y = dot(vec3<f32>(0.2989, 0.5886, 0.1145), c.rgb);
        let hit = select(y <= p1, y >= p1, kind > 5.5);
        if hit {
            c.a = c.a * p0;
        }
    }
    return c;
}
fn flag(f: LayerFilter, bit: u32) -> bool {
    return (u32(f.c.w) & bit) != 0u;
}
fn selected_mask_alpha(point: vec2<i32>, channel: u32, offset: f32, scale: f32) -> f32 {
    let dims=vec2<i32>(textureDimensions(mask_image));
    let sample=textureLoad(mask_image,clamp(point,vec2<i32>(0),dims-vec2<i32>(1)),0);
    let bytes=round(vec4<f32>(to_gamma(sample.rgb),sample.a)*255.0);
    return trunc(clamp(bytes[channel]*scale+offset,0.0,255.0))/255.0;
}
fn mask_alpha(uv: vec2<f32>, channel: u32, offset: f32, scale: f32) -> f32 {
    let point=uv*vec2<f32>(textureDimensions(mask_image))-vec2<f32>(0.5);
    let base=vec2<i32>(floor(point));let phase=fract(point);
    let a=selected_mask_alpha(base,channel,offset,scale);
    let b=selected_mask_alpha(base+vec2<i32>(1,0),channel,offset,scale);
    let c=selected_mask_alpha(base+vec2<i32>(0,1),channel,offset,scale);
    let d=selected_mask_alpha(base+vec2<i32>(1,1),channel,offset,scale);
    return mix(mix(a,b,phase.x),mix(c,d,phase.x),phase.y);
}
fn source_mask_texel(point: vec2<i32>, channel: u32, f: LayerFilter) -> f32 {
    let source_dims=vec2<i32>(textureDimensions(image,0));
    let source_point=clamp(point,vec2<i32>(0),source_dims-vec2<i32>(1));
    let original=textureLoad(image,source_point,0).a;
    let mask_dims=vec2<i32>(textureDimensions(mask_image));
    var mask_point=vec2<i32>(floor((vec2<f32>(source_point)+vec2<f32>(0.5))*vec2<f32>(mask_dims)/vec2<f32>(source_dims)));
    if f.g.w>0.5 {
        mask_point=source_point;
        if any(mask_point<vec2<i32>(0)) || any(mask_point>=mask_dims) {return original;}
    }
    let alpha=selected_mask_alpha(mask_point,channel,f.g.y,f.h.x);
    if f.g.z>0.5 {return floor(round(original*255.0)*alpha)/255.0;}
    return alpha;
}
fn source_mask_alpha(uv: vec2<f32>, channel: u32, f: LayerFilter) -> f32 {
    let point=uv*vec2<f32>(textureDimensions(image,0))-vec2<f32>(0.5);
    let base=vec2<i32>(floor(point));let phase=fract(point);
    let a=source_mask_texel(base,channel,f);
    let b=source_mask_texel(base+vec2<i32>(1,0),channel,f);
    let c=source_mask_texel(base+vec2<i32>(0,1),channel,f);
    let d=source_mask_texel(base+vec2<i32>(1,1),channel,f);
    return mix(mix(a,b,phase.x),mix(c,d,phase.x),phase.y);
}
//The ordered coverage is sampled in destination pixel space, independently of
//source UVs and LayerFilter rule masks. Stored byte arithmetic precedes tint.
fn ordered_source_alpha(alpha:f32,in:Out,f:LayerFilter)->f32 {
    if !flag(f,4096u) {return alpha;}
    let dimensions=vec2<i32>(textureDimensions(ordered_coverage,0));
    let at=clamp(vec2<i32>(floor(in.position.xy)),vec2<i32>(0),dimensions-vec2<i32>(1));
    let coverage=i32(round(to_gamma(textureLoad(ordered_coverage,at,0).rgb).b*255.0));
    let source_alpha=i32(round(clamp(alpha,0.0,1.0)*255.0));
    return f32((source_alpha*((coverage+1)>>1u))>>7u)/255.0;
}
fn filtered(in: Out) -> vec4<f32> {
    if outside_clip(in) { discard; }
    let f = filters[in.filter_index];
    let tint = vec4<f32>(to_gamma(in.color.rgb), select(in.color.a,1.0,flag(f,16u)));
    var sampled = distorted(bounded_uv(in.uv, in.sample_bounds), f);
    if (u32(f.i.w)&1u)!=0u { sampled.a=1.0; }
    sampled.a=ordered_source_alpha(sampled.a,in,f);
    var color = sampled * tint;
    if f.i.x>0.5 {
        let source_bytes=vec4<i32>(round(clamp(sampled,vec4<f32>(0.0),vec4<f32>(1.0))*255.0));
        let tint_bytes=vec4<i32>(round(clamp(tint,vec4<f32>(0.0),vec4<f32>(1.0))*255.0));
        let weight=(tint_bytes>>vec4<u32>(1u))+(tint_bytes>>vec4<u32>(7u));
        color=vec4<f32>((source_bytes*weight)>>vec4<u32>(7u))/255.0;
        if i32(f.i.x)==15 || i32(f.i.x)==19 || i32(f.i.x)==20 {
            //Scalar256 shares the RGB tint contract, but coverage keeps the
            //255 endpoint and uses an exact /256 product below that endpoint.
            var alpha=source_bytes.a;
            if tint_bytes.a<255 { alpha=(alpha*tint_bytes.a)>>8u; }
            color.a=f32(alpha)/255.0;
        } else if i32(f.i.x)==16 {
            color.a=f32(tint_bytes.a)/255.0;
        } else if i32(f.i.x)==17 || i32(f.i.x)==18 {
            // Paint colour alpha belongs to the source (including solids).
            // RGB-only profiles take their independent layer weight here.
            color.a=f32((u32(f.i.w)>>8u)&255u)/255.0;
        }
    }
    color = vec4<f32>(color.rgb * f.h.yzw, color.a);
    let org_a = color.a;
    let mono_y = dot(color.rgb, f.f.xyz);
    if flag(f, 1u) {
        var g = mix(color.rgb, vec3<f32>(mono_y), f.d.y);
        g = clamp(g, vec3<f32>(0.0), vec3<f32>(1.0));
        let row = f.d.x;
        let r = to_gamma(textureSampleLevel(curve_image, image_sampler, vec2<f32>(g.r, row), 0.0).rgb).r;
        let gg = to_gamma(textureSampleLevel(curve_image, image_sampler, vec2<f32>(g.g, row), 0.0).rgb).g;
        let b = to_gamma(textureSampleLevel(curve_image, image_sampler, vec2<f32>(g.b, row), 0.0).rgb).b;
        color = vec4<f32>(r, gg, b, color.a);
    }
    color = vec4<f32>(mix(color.rgb, vec3<f32>(1.0) - color.rgb, f.a.y), color.a);
    color = vec4<f32>(mix(color.rgb, vec3<f32>(mono_y), f.a.x), color.a);
    color = vec4<f32>(color.rgb + vec3<f32>(f.a.z) - vec3<f32>(f.a.w), color.a);
    color = vec4<f32>(mix(color.rgb, f.b.yzw, f.b.x), color.a);
    color = vec4<f32>(color.rgb + f.c.xyz, org_a);
    let final_gray = dot(color.rgb, vec3<f32>(0.2989, 0.5886, 0.1145));
    if flag(f, 2u) {
        let m = textureSampleLevel(mask_image, image_sampler, in.uv, 0.0);
        let encoded_mask=vec4<f32>(to_gamma(m.rgb),m.a);
        if f.g.x<0.5 {
            color=color*encoded_mask;
        } else {
            let channel=u32(f.g.x)-1u;
            if f.g.z>0.5 || f.g.w>0.5 {
                let alpha=ordered_source_alpha(source_mask_alpha(bounded_uv(in.uv,in.sample_bounds),channel,f),in,f);
                if f.i.x>0.5 {
                    let tint_byte=i32(round(tint.a*255.0));
                    color.a=f32((i32(round(alpha*255.0))*((tint_byte>>1u)+(tint_byte>>7u)))>>7u)/255.0;
                }else{color.a=alpha*tint.a;}
            } else {color.a=ordered_source_alpha(mask_alpha(in.uv,channel,f.g.y,f.h.x),in,f);}
        }
    }
    if flag(f, 4u) {
        color.a = ordered_source_alpha(final_gray,in,f);
    }
    if flag(f, 8u) && color.a < 1.0 / 255.0 {
        discard;
    }
    if f.f.w > 0.0 {color=vec4<f32>(color.rgb*pow(clamp(color.a,0.0,1.0),f.f.w),color.a);}
    return vec4<f32>(to_linear(color.rgb), clamp(color.a, 0.0, 1.0));
}
@fragment fn fs_filtered(in: Out) -> @location(0) vec4<f32> {
    return filtered(in);
}
@fragment fn fs_filtered_premultiply(in: Out) -> @location(0) vec4<f32> {
    let c = filtered(in);
    return vec4<f32>(c.rgb * c.a, c.a);
}

// Integer equivalents of crysirror_image::ConstantByteBlend::channel.
fn stored_channel(d:i32,s:i32,a:i32,mode:i32,mask_weight:i32)->i32 {
    let full=a+(a>>7u); let half=(a>>1u)+(a>>7u);
    let positive=(s*full)>>8u;
    let white=255+(((s-255)*half)>>7u);
    let middle=128+(((s-128)*half)>>7u);
    var value=d;
    switch mode {
        case 1: {value=d+(((s-d)*((a+1)>>1u))>>7u);}
        case 2: {value=positive+d-((positive*d)>>8u);}
        case 3: {value=((white+(white>>7u))*d)>>8u;}
        case 4: {value=abs(positive-d);}
        case 5: {value=max(positive,d);}
        case 6: {value=min(white,d);}
        case 7: {value=positive+d;}
        case 8: {value=d-positive;}
        case 9: {value=d-(((255-s)*full)>>8u);}
        case 10,11: {
            let high=select(middle>128,d>128,mode==10);
            if high {value=max(2*(middle+d-((middle*d)>>8u))-255,0);}
            else {value=(2*middle*d)>>8u;}
        }
        case 12: {value=positive+d-2*((positive*d)>>8u);}
        case 13: {value=2*middle+d-255;}
        case 14: {
            if middle>128 {value=max(2*middle-255,d);}
            else {value=min(2*middle,d);}
        }
        case 15: {value=d+(((s-d)*a)>>8u);}
        case 16: {
            if a==255 {value=s;} else {value=d+(((s-d)*a)>>8u);}
        }
        case 17: {
            let source=(s*a)/255;
            value=255-((255-d)*(255-source))/255;
        }
        case 18: {value=(d*s)/255;}
        case 19: {
            if a==255 {value=s;}
            else if a!=0 {value=((s*a)>>8u)+((d*(256-a))>>8u);}
        }
        case 20: {
            if a!=0 && mask_weight!=0 {
                var x=mask_weight; var y=256-x;
                if a!=255 {x=(a*mask_weight)>>8u;y=255-x;}
                value=(s*x+d*y)>>8u;
            }
        }
        default: {}
    }
    return clamp(value,0,255);
}
@fragment fn fs_stored(in:Out)->@location(0) vec4<f32> {
    let f=filters[in.filter_index];
    let source=filtered(in);
    let size=vec2<i32>(textureDimensions(destination_image,0));
    let at=clamp(vec2<i32>(floor(in.position.xy))-vec2<i32>(f.i.yz),vec2<i32>(0),size-vec2<i32>(1));
    let destination=textureLoad(destination_image,at,0);
    let s=vec3<i32>(round(to_gamma(source.rgb)*255.0));
    let d=vec3<i32>(round(to_gamma(destination.rgb)*255.0));
    let alpha=i32(round(source.a*255.0));
    let mode=i32(f.i.x);
    let mask_weight=i32(round(fract(f.i.x)*512.0));
    let rgb=vec3<f32>(f32(stored_channel(d.r,s.r,alpha,mode,mask_weight)),
        f32(stored_channel(d.g,s.g,alpha,mode,mask_weight)),f32(stored_channel(d.b,s.b,alpha,mode,mask_weight)))/255.0;
    let output_alpha=select(destination.a,f32(alpha)/255.0,mode==15);
    return vec4<f32>(to_linear(rgb),select(output_alpha,1.0,(u32(f.i.w)&2u)!=0u));
}
