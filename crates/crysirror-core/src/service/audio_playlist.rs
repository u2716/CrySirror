//! Portable segmented audio program. Encoded assets remain immutable; no VM,
//! decoder, device or expanded repeat buffer is part of this contract.
use super::EncodedAudio;
use alloc::vec::Vec;
#[derive(Clone, Debug)]
pub enum AudioPlaylistNode {
    Clip(u16),
    Sequence(Vec<AudioPlaylistNode>),
    /// None repeats forever, Some(n) plays n complete iterations.
    Repeat {
        count: Option<u32>,
        body: Vec<AudioPlaylistNode>,
    },
}
#[derive(Clone, Debug)]
pub struct AudioPlaylist {
    pub clips: Vec<EncodedAudio>,
    pub program: AudioPlaylistNode,
}
impl AudioPlaylist {
    pub fn encoded_bytes(&self) -> crate::Result<usize> {
        if self.clips.is_empty() || self.clips.len() > 64 {
            return Err(crate::Error::Limit("audio playlist clips"));
        }
        let mut count = 0;
        fn node(
            n: &AudioPlaylistNode,
            depth: usize,
            count: &mut usize,
            clips: usize,
        ) -> crate::Result<()> {
            *count += 1;
            if depth > 16 || *count > 1024 {
                return Err(crate::Error::Limit("audio playlist program"));
            }
            match n {
                AudioPlaylistNode::Clip(i) if usize::from(*i) < clips => Ok(()),
                AudioPlaylistNode::Clip(_) => {
                    Err(crate::Error::Unsupported("audio playlist clip index"))
                }
                AudioPlaylistNode::Sequence(v) | AudioPlaylistNode::Repeat { body: v, .. } => {
                    if v.is_empty() || matches!(n, AudioPlaylistNode::Repeat { count: Some(0), .. })
                    {
                        return Err(crate::Error::Unsupported("empty audio playlist group"));
                    }
                    for n in v {
                        node(n, depth + 1, count, clips)?;
                    }
                    Ok(())
                }
            }
        }
        node(&self.program, 0, &mut count, self.clips.len())?;
        self.clips.iter().try_fold(0usize, |sum, clip| {
            sum.checked_add(clip.bytes.len())
                .filter(|n| *n <= 256 * 1024 * 1024)
                .ok_or(crate::Error::Limit("audio playlist encoded bytes"))
        })
    }
}
