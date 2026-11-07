// GPL-2.0-or-later. Complete native AB50 raw assembly and3510 allocation,
//CRT srand/rand fill, strict threshold comparison and cleanup inspected.
//! Native table/clock policy over the shared tile-reveal image operation.
use super::*;
pub(super) struct BlockMask {
    grid: [u16; 2],
    values: Vec<u8>,
}
impl BlockMask {
    pub(super) fn new(size: [u32; 2], rng: &mut SequenceRng) -> Result<Self> {
        ensure!(
            size.iter().all(|n| (1..=8192).contains(n))
                && u64::from(size[0]) * u64::from(size[1]) <= 4_194_304,
            "native block mask extent/budget"
        );
        let grid = size.map(|n| n.div_ceil(16) as u16);
        let padded = size.map(|n| n.div_ceil(16) * 16);
        let mut values = Vec::with_capacity(usize::from(grid[0]) * usize::from(grid[1]));
        let mut consumed = 0u64;
        //Original allocates/fills rounded PIXEL dimensions, but reads cell
        //(x/16,y/16). This is not a compact tile-sized random stream. Shared
        //LCG jump-ahead consumes all gaps exactly without materializing the
        //unused native table or running a canvas-sized loop on the owner.
        for y in 0..u64::from(grid[1]) {
            for x in 0..u64::from(grid[0]) {
                let index = y * u64::from(padded[0]) + x;
                rng.advance(index - consumed)?;
                values.push(rng.next_u32()? as u8);
                consumed = index + 1;
            }
        }
        rng.advance(u64::from(padded[0]) * u64::from(padded[1]) - consumed)?;
        Ok(Self { grid, values })
    }
    pub(super) fn sample(
        &self,
        time: u32,
        duration: u32,
        old: Arc<dyn ImageResource>,
        new: Arc<dyn ImageResource>,
    ) -> Result<ImageInput> {
        ensure!(
            duration > 0 && time < duration && old.size() == new.size(),
            "native block transition frame/clock"
        );
        let value = 255u64 * u64::from(time) / u64::from(duration);
        Ok(ImageInput::Process {
            input: Box::new(ImageInput::Raster {
                destination: Box::new(ImageInput::Resource(old)),
                source: Box::new(ImageInput::Resource(new)),
            }),
            effects: serde_json::to_vec(&json!([{"kind":"tile_reveal","spec":{
                "cell":[16,16],"grid":self.grid,"thresholds":self.values,"value":value
            }}]))?
            .into(),
        })
    }
}
