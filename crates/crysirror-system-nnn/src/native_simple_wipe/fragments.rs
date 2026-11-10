// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native Hahen vertex/face data and trajectory policy from effectSimpleWipeHahen.
//! Generic polygon rasterization belongs to shared Image.
use super::*;
const VERTICES: [[i32; 2]; 66] = [
    [0, 0],
    [90, 0],
    [215, 0],
    [255, 0],
    [421, 0],
    [511, 0],
    [639, 0],
    [0, 70],
    [166, 65],
    [188, 33],
    [251, 50],
    [320, 102],
    [390, 72],
    [494, 56],
    [483, 86],
    [460, 58],
    [639, 87],
    [0, 136],
    [26, 121],
    [123, 175],
    [188, 146],
    [288, 169],
    [480, 177],
    [576, 155],
    [639, 197],
    [0, 188],
    [80, 242],
    [109, 249],
    [180, 278],
    [266, 272],
    [297, 224],
    [356, 265],
    [444, 235],
    [538, 286],
    [639, 249],
    [0, 259],
    [0, 309],
    [40, 343],
    [114, 338],
    [274, 330],
    [393, 321],
    [483, 347],
    [547, 331],
    [0, 385],
    [91, 411],
    [208, 402],
    [315, 381],
    [425, 379],
    [372, 408],
    [493, 435],
    [553, 377],
    [639, 429],
    [0, 479],
    [22, 479],
    [47, 450],
    [96, 456],
    [121, 445],
    [93, 479],
    [215, 479],
    [325, 466],
    [309, 479],
    [414, 479],
    [453, 479],
    [536, 479],
    [550, 479],
    [639, 479],
];
const FACES: [&[usize]; 32] = [
    &[0, 7, 18, 19, 20, 8, 1],
    &[1, 8, 2],
    &[2, 9, 10, 3],
    &[3, 10, 11, 12, 4],
    &[4, 12, 13, 5],
    &[5, 14, 22, 23, 16, 6],
    &[13, 15, 14],
    &[7, 17, 18],
    &[9, 8, 20, 21, 11, 10],
    &[15, 12, 11, 21, 30, 31, 32, 22, 14],
    &[16, 23, 24],
    &[18, 17, 25, 26, 27, 19],
    &[20, 19, 27, 28, 29, 30, 21],
    &[23, 22, 32, 33, 34, 24],
    &[25, 35, 26],
    &[26, 35, 36, 37, 38, 28, 27],
    &[29, 28, 38, 44, 56, 45, 39],
    &[30, 29, 39, 46, 47, 40, 31],
    &[32, 31, 40, 41, 42, 33],
    &[34, 33, 50, 51],
    &[36, 43, 37],
    &[38, 37, 43, 52, 53, 44],
    &[44, 54, 55, 56],
    &[54, 53, 57, 55],
    &[45, 55, 57, 58],
    &[39, 45, 58, 60, 59, 46],
    &[47, 46, 59, 48],
    &[48, 60, 61],
    &[40, 48, 61, 62, 49, 41],
    &[49, 62, 63],
    &[42, 41, 49, 63, 64, 50],
    &[50, 64, 65, 51],
];
pub(super) fn plan(
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    count: u32,
    duration: u32,
    reverse: bool,
) -> Result<(ImageInput, Vec<ImageEffect>)> {
    let (old, new, c) = if reverse {
        (new, old, duration - count.min(duration))
    } else {
        (old, new, count.min(duration))
    };
    let [w, h] = old.size();
    let w = i64::from(w);
    let h = i64::from(h);
    let vertices = VERTICES.map(|[x, y]| {
        [
            (i64::from(x) * w / 640).min(w - 1) as i32,
            (i64::from(y) * h / 480).min(h - 1) as i32,
        ]
    });
    let limit = duration * 20 / 100;
    let mut fragments = Vec::new();
    for face in FACES {
        let cx = face.iter().map(|i| i64::from(vertices[*i][0])).sum::<i64>() / face.len() as i64;
        let cy = face.iter().map(|i| i64::from(vertices[*i][1])).sum::<i64>() / face.len() as i64;
        let x = cx - w / 2;
        let y = cy - h / 2;
        let distance = ((x * x + y * y) as f64).sqrt().max(1.);
        let speed = 30. * (1. + distance / 200.);
        let angle = (y as f64).atan2(x as f64);
        let vx = (angle.cos() * speed) as f32;
        let vy = (angle.sin() * speed) as f32;
        let mut phase = 10f32;
        phase *= 0.001f32;
        phase *= if c <= limit { c as f32 } else { limit as f32 };
        let mut dx = vx * phase;
        let mut dy = vy * phase;
        if c > limit {
            let mut scale = ((w * w + h * h) as f64).sqrt() as f32;
            scale = (f64::from(scale) / 30.) as f32;
            scale *= 0.3f32;
            let divisor = (duration - limit).max(1) as f32;
            dx += vx * (c - limit) as f32 * scale / divisor;
            dy += vy * (c - limit) as f32 * scale / divisor;
        }
        fragments.push(crysirror_image::integer_wipe::fragments::ImageFragment {
            vertices: face.iter().map(|i| vertices[*i]).collect(),
            translation: [(dx + 0.5f32) as i32, (dy + 0.5f32) as i32],
        });
    }
    Ok((
        ImageInput::Raster {
            destination: Box::new(ImageInput::Resource(new)),
            source: Box::new(ImageInput::Resource(old)),
        },
        vec![ImageEffect::ByteRaster {
            spec: ByteRaster::PolygonFragments {
                spec: crysirror_image::integer_wipe::fragments::PolygonFragments { fragments },
            },
        }],
    ))
}
