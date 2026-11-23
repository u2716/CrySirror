//! Original6D5218 body sections. These are native stream records, not a host
//! checkpoint: names remain UTF-16 identifiers, scalar state remains in its
//! original order, and no executable/device pointer is reconstructed.
//!
//! Order:6E9BF0 thread/routine/block tree;6BBE4C reader;6CC598 paths;
//! 5D3B58 text log;5C44E0 sound;two5CF8CC image records;6B8D8C graphic;
//! 6CB5F8 screen objects;6C7B68 camera;6C3E78 calculator;optional callback;
//!419-byte script state;seven strings;label/file logs;temporary-save store;
//!69A660 input;6A7B78 camera matrix;eight metadata strings.
use super::*;

/// Preserve native UTF-16 exactly, including code units a Delphi string can
/// contain but a Rust String cannot. Resource resolution may later validate it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Text(pub Vec<u16>);
#[derive(Clone, Debug)]
pub(super) struct Reader<'a> {
    source: &'a [u8],
    pub position: usize,
}
impl<'a> Reader<'a> {
    pub fn new(source: &'a [u8]) -> Result<Self> {
        ensure!(source.len() as u64 <= LIMIT, "QLIE native body budget");
        Ok(Self {
            source,
            position: 0,
        })
    }
    pub fn bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(len)
            .context("QLIE native body offset")?;
        ensure!(
            end <= self.source.len(),
            "truncated QLIE native section at {:#x}",
            self.position
        );
        let bytes = &self.source[self.position..end];
        self.position = end;
        Ok(bytes)
    }
    pub fn byte(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }
    pub fn int(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.bytes(4)?.try_into()?))
    }
    pub fn count(&mut self, max: usize) -> Result<usize> {
        let count = usize::try_from(self.int()?).context("negative QLIE native section count")?;
        ensure!(count <= max, "QLIE native section count budget");
        Ok(count)
    }
    pub fn text(&mut self) -> Result<Text> {
        let count = usize::try_from(i16::from_le_bytes(self.bytes(2)?.try_into()?))
            .context("negative QLIE native string length")?;
        Ok(Text(
            self.bytes(count * 2)?
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]))
                .collect(),
        ))
    }
    fn texts(&mut self, count: usize) -> Result<Vec<Text>> {
        (0..count).map(|_| self.text()).collect()
    }
    fn ints(&mut self, count: usize) -> Result<Vec<i32>> {
        (0..count).map(|_| self.int()).collect()
    }
    fn header(&mut self, tag: &[u8; 8]) -> Result<()> {
        ensure!(self.bytes(8)? == tag, "QLIE native section tag");
        let version = self.int()?;
        ensure!(
            version == 10,
            "unsupported QLIE native reader section version {version}"
        );
        Ok(())
    }
    pub fn remaining(&self) -> &'a [u8] {
        &self.source[self.position..]
    }
}
fn put_int(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_count(out: &mut Vec<u8>, count: usize) -> Result<()> {
    put_int(out, i32::try_from(count)?);
    Ok(())
}
fn put_text(out: &mut Vec<u8>, text: &Text) -> Result<()> {
    out.extend_from_slice(&i16::try_from(text.0.len())?.to_le_bytes());
    for unit in &text.0 {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    Ok(())
}
fn put_texts(out: &mut Vec<u8>, texts: &[Text]) -> Result<()> {
    for text in texts {
        put_text(out, text)?;
    }
    Ok(())
}
fn put_ints(out: &mut Vec<u8>, values: &[i32]) {
    for &value in values {
        put_int(out, value);
    }
}

#[derive(Clone, Debug)]
pub(super) struct Block {
    pub names: [Text; 2],
    pub mode: u8,
    pub counters: [i32; 2],
    pub arguments: [Text; 8],
}
impl Block {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        ensure!(r.int()? == 10, "QLIE native block version");
        let names = r.texts(2)?.try_into().unwrap();
        let mode = r.byte()?;
        let counters = r.ints(2)?.try_into().unwrap();
        let arguments = r.texts(8)?.try_into().unwrap();
        Ok(Self {
            names,
            mode,
            counters,
            arguments,
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        put_int(out, 10);
        put_texts(out, &self.names)?;
        out.push(self.mode);
        put_ints(out, &self.counters);
        put_texts(out, &self.arguments)
    }
}
///6EA164 optional+104 calculator/call receipt. Integer/string/object banks
///have independent positions even when an object name happens to be empty.
#[derive(Clone, Debug)]
pub(super) struct CallValues {
    pub integers: [i32; 16],
    pub strings: [Text; 16],
    pub objects: [Text; 16],
    pub scope: Text,
    pub button_integers: [i32; 4],
    pub button_strings: [Text; 4],
    pub state: [i32; 2],
    pub sub_integers: [i32; 4],
    pub sub_strings: [Text; 4],
}
impl CallValues {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            integers: r.ints(16)?.try_into().unwrap(),
            strings: r.texts(16)?.try_into().unwrap(),
            objects: r.texts(16)?.try_into().unwrap(),
            scope: r.text()?,
            button_integers: r.ints(4)?.try_into().unwrap(),
            button_strings: r.texts(4)?.try_into().unwrap(),
            state: r.ints(2)?.try_into().unwrap(),
            sub_integers: r.ints(4)?.try_into().unwrap(),
            sub_strings: r.texts(4)?.try_into().unwrap(),
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        put_ints(out, &self.integers);
        put_texts(out, &self.strings)?;
        put_texts(out, &self.objects)?;
        put_text(out, &self.scope)?;
        put_ints(out, &self.button_integers);
        put_texts(out, &self.button_strings)?;
        put_ints(out, &self.state);
        put_ints(out, &self.sub_integers);
        put_texts(out, &self.sub_strings)
    }
}
#[derive(Clone, Debug)]
pub(super) struct Routine {
    pub names: [Text; 16],
    pub mode: u8,
    pub counters: [i32; 3],
    pub location: [Text; 3],
    pub call: Option<CallValues>,
    pub blocks: Vec<Block>,
}
impl Routine {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        r.header(b"routine\0")?;
        let names = r.texts(16)?.try_into().unwrap();
        let mode = r.byte()?;
        let counters = r.ints(3)?.try_into().unwrap();
        let location = r.texts(3)?.try_into().unwrap();
        let call = if r.byte()? != 0 {
            Some(CallValues::read(r)?)
        } else {
            None
        };
        let count = r.count(1024)?;
        let blocks = (0..count).map(|_| Block::read(r)).collect::<Result<_>>()?;
        Ok(Self {
            names,
            mode,
            counters,
            location,
            call,
            blocks,
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        out.extend_from_slice(b"routine\0");
        put_int(out, 10);
        put_texts(out, &self.names)?;
        out.push(self.mode);
        put_ints(out, &self.counters);
        put_texts(out, &self.location)?;
        out.push(u8::from(self.call.is_some()));
        if let Some(call) = &self.call {
            call.write(out)?;
        }
        put_count(out, self.blocks.len())?;
        for block in &self.blocks {
            block.write(out)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(super) struct Thread {
    pub routines: Vec<Routine>,
}
impl Thread {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        r.header(b"thread\0\0")?;
        let count = r.count(1024)?;
        Ok(Self {
            routines: (0..count)
                .map(|_| Routine::read(r))
                .collect::<Result<_>>()?,
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        out.extend_from_slice(b"thread\0\0");
        put_int(out, 10);
        put_count(out, self.routines.len())?;
        for routine in &self.routines {
            routine.write(out)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(super) struct ReplaceRule {
    pub find: Text,
    pub replace: Text,
    pub flags: i32,
}
///6BBE4C:77 bytes from reader+76 followed by raw text/file/label and two
///retained identifiers, then replacement rules (two strings plus int each).
#[derive(Clone, Debug)]
pub(super) struct ScriptReader {
    pub state: [u8; 77],
    pub identifiers: [Text; 5],
    pub replacements: Vec<ReplaceRule>,
}
impl ScriptReader {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let state: [u8; 77] = r.bytes(77)?.try_into().unwrap();
        ensure!(
            i32::from_le_bytes(state[..4].try_into()?) == 10,
            "QLIE native reader state version"
        );
        let identifiers = r.texts(5)?.try_into().unwrap();
        let count = r.count(65536)?;
        let replacements = (0..count)
            .map(|_| {
                Ok(ReplaceRule {
                    find: r.text()?,
                    replace: r.text()?,
                    flags: r.int()?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            state,
            identifiers,
            replacements,
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        out.extend_from_slice(&self.state);
        put_texts(out, &self.identifiers)?;
        put_count(out, self.replacements.len())?;
        for rule in &self.replacements {
            put_text(out, &rule.find)?;
            put_text(out, &rule.replace)?;
            put_int(out, rule.flags);
        }
        Ok(())
    }
}

///6CC598 uses a signed32-bit string count, then one mode byte after
///the UTF-16 path list. It never serializes a host filesystem handle.
#[derive(Clone, Debug)]
pub(super) struct Paths {
    pub entries: Vec<Text>,
    pub mode: u8,
}
impl Paths {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let count = r.count(65535)?;
        Ok(Self {
            entries: r.texts(count)?,
            mode: r.byte()?,
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        put_count(out, self.entries.len())?;
        put_texts(out, &self.entries)?;
        out.push(self.mode);
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(super) struct TextLogEntry {
    pub strings: [Text; 4],
    pub integers: [i32; 4],
}
///5D3B58 always stores all256 ring entries, even when only a short prefix
///is populated. Header fields+4/+8 retain the native ring position/count.
#[derive(Clone, Debug)]
pub(super) struct TextLog {
    pub header: [u8; 76],
    pub entries: Vec<TextLogEntry>,
}
impl TextLog {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let header: [u8; 76] = r.bytes(76)?.try_into()?;
        ensure!(
            i32::from_le_bytes(header[..4].try_into()?) == 10,
            "QLIE native text log version"
        );
        let entries = (0..256)
            .map(|_| {
                Ok(TextLogEntry {
                    strings: r.texts(4)?.try_into().unwrap(),
                    integers: r.ints(4)?.try_into().unwrap(),
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self { header, entries })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        ensure!(self.entries.len() == 256, "QLIE native text log ring size");
        out.extend_from_slice(&self.header);
        for entry in &self.entries {
            put_texts(out, &entry.strings)?;
            put_ints(out, &entry.integers);
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(super) struct DiscreteSound {
    pub state: [u8; 86],
    pub identifier: Text,
}
#[derive(Clone, Debug)]
pub(super) struct StreamSound {
    pub state: [u8; 131],
    pub identifiers: [Text; 2],
}
///5C44E0 writes DSND then SSND. Numeric playback state is packed, so the
///unaligned offsets below are native file offsets, never Rust struct layout.
///DSND+4 is current position,+8 playing; SSND+4 position,+16 playing,
///+21 float rate,+29 native state; the filename pair follows its131 bytes.
#[derive(Clone, Debug)]
pub(super) struct Sound {
    pub discrete: Vec<DiscreteSound>,
    pub streams: Vec<StreamSound>,
}
impl Sound {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        ensure!(
            r.bytes(8)? == b"dsnd\0\0\0\0",
            "QLIE native discrete audio tag"
        );
        let count = r.count(4096)?;
        let discrete = (0..count)
            .map(|_| {
                let state: [u8; 86] = r.bytes(86)?.try_into()?;
                ensure!(
                    i32::from_le_bytes(state[..4].try_into()?) == 10,
                    "QLIE native discrete audio version"
                );
                Ok(DiscreteSound {
                    state,
                    identifier: r.text()?,
                })
            })
            .collect::<Result<_>>()?;
        ensure!(
            r.bytes(8)? == b"ssnd\0\0\0\0",
            "QLIE native stream audio tag"
        );
        let count = r.count(4096)?;
        let streams = (0..count)
            .map(|_| {
                let state: [u8; 131] = r.bytes(131)?.try_into()?;
                ensure!(
                    i32::from_le_bytes(state[..4].try_into()?) == 10,
                    "QLIE native stream audio version"
                );
                Ok(StreamSound {
                    state,
                    identifiers: r.texts(2)?.try_into().unwrap(),
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self { discrete, streams })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        out.extend_from_slice(b"dsnd\0\0\0\0");
        put_count(out, self.discrete.len())?;
        for sound in &self.discrete {
            out.extend_from_slice(&sound.state);
            put_text(out, &sound.identifier)?;
        }
        out.extend_from_slice(b"ssnd\0\0\0\0");
        put_count(out, self.streams.len())?;
        for sound in &self.streams {
            out.extend_from_slice(&sound.state);
            put_texts(out, &sound.identifiers)?;
        }
        Ok(())
    }
}

///One native33-entry image replacement slot: original identifier then an
///optional encoded image stream. Pixel codecs remain shared engine resources.
#[derive(Clone, Debug)]
pub(super) struct ImageMember {
    pub identifier: Text,
    pub encoded: Vec<u8>,
}
///5C9948 writes a56-byte `vgene` record. Its four bytes at+8 came from an
///uninitialized native stack local (v5), not a validated format version.
///We preserve read bytes; any newly authored record must initialize them to0.
#[derive(Clone, Debug)]
pub(super) struct GeneratorRecord {
    pub bytes: [u8; 56],
}
impl GeneratorRecord {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let bytes: [u8; 56] = r.bytes(56)?.try_into()?;
        ensure!(
            &bytes[..8] == b"vgene\0\0\0",
            "QLIE native generator record tag"
        );
        Ok(Self { bytes })
    }
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.bytes);
    }
}
#[derive(Clone, Debug)]
pub(super) struct StringGenerators {
    pub source: Text,
    pub generators: Vec<GeneratorRecord>,
}
impl StringGenerators {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let source = r.text()?;
        //5CBB60 splits definitions at backslashes,5CB6A4 allocates one
        //TValueGenerator for every nonempty definition, including mode:none.
        let count = source
            .0
            .split(|unit| *unit == u16::from(b'\\'))
            .filter(|part| {
                part.iter()
                    .any(|unit| char::from_u32(u32::from(*unit)).is_none_or(|c| !c.is_whitespace()))
            })
            .count();
        ensure!(count <= 252, "QLIE native string generator count");
        let generators = (0..count)
            .map(|_| GeneratorRecord::read(r))
            .collect::<Result<_>>()?;
        Ok(Self { source, generators })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        put_text(out, &self.source)?;
        for record in &self.generators {
            record.write(out);
        }
        Ok(())
    }
}
///5CF8CC v11 starts with2654 packed bytes, retaining both256-entry
///animation integer banks and native pose/wave/timing/skip state. These are
///file records; no native memory address is dereferenced by this codec.
#[derive(Clone, Debug)]
pub(super) struct Image {
    pub state: Vec<u8>,
    pub resource: Text,
    pub members: Vec<ImageMember>,
    pub strings536: [Text; 32],
    pub strings408: [Text; 32],
    pub jump_table: [u8; 256],
    pub generators: [GeneratorRecord; 4],
    pub string_generators: StringGenerators,
    pub user_integers: [i32; 8],
    pub user_strings: [Text; 8],
    pub name: Text,
}
impl Image {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let state = r.bytes(2654)?.to_vec();
        ensure!(
            i32::from_le_bytes(state[..4].try_into()?) == 11,
            "unsupported QLIE native image state version"
        );
        let resource = r.text()?;
        let members = (0..33)
            .map(|_| {
                let identifier = r.text()?;
                let size = r.count(LIMIT as usize)?;
                let encoded = r.bytes(size)?.to_vec();
                Ok(ImageMember {
                    identifier,
                    encoded,
                })
            })
            .collect::<Result<_>>()?;
        let strings536 = r.texts(32)?.try_into().unwrap();
        let strings408 = r.texts(32)?.try_into().unwrap();
        let jump_table = r.bytes(256)?.try_into()?;
        let generators = (0..4)
            .map(|_| GeneratorRecord::read(r))
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .unwrap();
        let string_generators = StringGenerators::read(r)?;
        let mut user_integers = [0; 8];
        let mut user_strings = std::array::from_fn(|_| Text::default());
        for i in 0..8 {
            user_integers[i] = r.int()?;
            user_strings[i] = r.text()?;
        }
        let name = r.text()?;
        Ok(Self {
            state,
            resource,
            members,
            strings536,
            strings408,
            jump_table,
            generators,
            string_generators,
            user_integers,
            user_strings,
            name,
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        ensure!(
            self.state.len() == 2654 && self.members.len() == 33,
            "QLIE native image state/member layout"
        );
        out.extend_from_slice(&self.state);
        put_text(out, &self.resource)?;
        for member in &self.members {
            put_text(out, &member.identifier)?;
            put_count(out, member.encoded.len())?;
            out.extend_from_slice(&member.encoded);
        }
        put_texts(out, &self.strings536)?;
        put_texts(out, &self.strings408)?;
        out.extend_from_slice(&self.jump_table);
        for record in &self.generators {
            record.write(out);
        }
        self.string_generators.write(out)?;
        for i in 0..8 {
            put_int(out, self.user_integers[i]);
            put_text(out, &self.user_strings[i])?;
        }
        put_text(out, &self.name)
    }
}

///6B8D8C writes140 state bytes,+20 identifier, then only captures whose
///native presence bytes101/102 (file offsets33/34) are set. Mode9 embeds
///one additional5CF8CC image. Capture pixels stay encoded and source-owned.
#[derive(Clone, Debug)]
pub(super) struct Graphic {
    pub state: [u8; 140],
    pub identifier: Text,
    pub captures: [Option<ImageMember>; 2],
    pub animation: Option<Image>,
}
impl Graphic {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let state: [u8; 140] = r.bytes(140)?.try_into()?;
        ensure!(
            i32::from_le_bytes(state[..4].try_into()?) == 10,
            "QLIE native graphic version"
        );
        let identifier = r.text()?;
        let mut captures = [None, None];
        for i in 0..2 {
            if state[33 + i] != 0 {
                let identifier = r.text()?;
                let size = r.count(LIMIT as usize)?;
                captures[i] = Some(ImageMember {
                    identifier,
                    encoded: r.bytes(size)?.to_vec(),
                });
            }
        }
        let animation = if state[9] == 9 {
            Some(Image::read(r)?)
        } else {
            None
        };
        Ok(Self {
            state,
            identifier,
            captures,
            animation,
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        out.extend_from_slice(&self.state);
        put_text(out, &self.identifier)?;
        for i in 0..2 {
            ensure!(
                (self.state[33 + i] != 0) == self.captures[i].is_some(),
                "QLIE native capture flag/body mismatch"
            );
            if let Some(capture) = &self.captures[i] {
                put_text(out, &capture.identifier)?;
                put_count(out, capture.encoded.len())?;
                out.extend_from_slice(&capture.encoded);
            }
        }
        ensure!(
            (self.state[9] == 9) == self.animation.is_some(),
            "QLIE native effect image flag/body mismatch"
        );
        if let Some(image) = &self.animation {
            image.write(out)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(super) struct ScreenObject {
    pub kind: i32,
    pub identifiers: [Text; 4],
}
#[derive(Clone, Debug)]
pub(super) struct ScreenSlot {
    pub identifier: Text,
    pub integers: [i32; 3],
}
///6CB5F8 screen-manager metadata. Its typed resource identifiers are separate
///from the calculator's actual object payloads later in the body.
#[derive(Clone, Debug)]
pub(super) struct Screen {
    pub state: [u8; 69],
    pub objects: Vec<ScreenObject>,
    pub active: i32,
    pub slots: [ScreenSlot; 27],
}
impl Screen {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let state: [u8; 69] = r.bytes(69)?.try_into()?;
        ensure!(
            i32::from_le_bytes(state[..4].try_into()?) == 10,
            "QLIE native screen-manager version"
        );
        let count = r.count(65536)?;
        let objects = (0..count)
            .map(|_| {
                Ok(ScreenObject {
                    kind: r.int()?,
                    identifiers: r.texts(4)?.try_into().unwrap(),
                })
            })
            .collect::<Result<_>>()?;
        let active = r.int()?;
        let slots = (0..27)
            .map(|_| {
                Ok(ScreenSlot {
                    identifier: r.text()?,
                    integers: r.ints(3)?.try_into().unwrap(),
                })
            })
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .unwrap();
        Ok(Self {
            state,
            objects,
            active,
            slots,
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        out.extend_from_slice(&self.state);
        put_count(out, self.objects.len())?;
        for object in &self.objects {
            put_int(out, object.kind);
            put_texts(out, &object.identifiers)?;
        }
        put_int(out, self.active);
        for slot in &self.slots {
            put_text(out, &slot.identifier)?;
            put_ints(out, &slot.integers);
        }
        Ok(())
    }
}
///6C7B68 stores141 bytes from TCamera+4, including unaligned native fields.
///Raw zero defaults are preserved here;6C7AFC applies its runtime1000 defaults
///after loading, which is a restore policy rather than an encoding mutation.
#[derive(Clone, Debug)]
pub(super) struct Camera {
    pub state: [u8; 141],
}
impl Camera {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            state: r.bytes(141)?.try_into()?,
        })
    }
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.state);
    }
}

///Standalone codec for the leading reader sections. `consumed` is the exact
///offset of the calculator section; it is never guessed by scanning.
#[derive(Clone, Debug)]
pub(super) struct ReaderSections {
    pub thread: Thread,
    pub reader: ScriptReader,
    pub paths: Paths,
    pub text_log: TextLog,
    pub sound: Sound,
    pub images: [Image; 2],
    pub graphic: Graphic,
    pub screen: Screen,
    pub camera: Camera,
    pub consumed: usize,
}
impl ReaderSections {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes)?;
        let thread = Thread::read(&mut r)?;
        let reader = ScriptReader::read(&mut r)?;
        let paths = Paths::read(&mut r)?;
        let text_log = TextLog::read(&mut r)?;
        let sound = Sound::read(&mut r)?;
        let images = [Image::read(&mut r)?, Image::read(&mut r)?];
        let graphic = Graphic::read(&mut r)?;
        let screen = Screen::read(&mut r)?;
        let camera = Camera::read(&mut r)?;
        Ok(Self {
            thread,
            reader,
            paths,
            text_log,
            sound,
            images,
            graphic,
            screen,
            camera,
            consumed: r.position,
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        self.thread.write(&mut out)?;
        self.reader.write(&mut out)?;
        self.paths.write(&mut out)?;
        self.text_log.write(&mut out)?;
        self.sound.write(&mut out)?;
        for image in &self.images {
            image.write(&mut out)?;
        }
        self.graphic.write(&mut out)?;
        self.screen.write(&mut out)?;
        self.camera.write(&mut out);
        ensure!(
            out.len() as u64 <= LIMIT,
            "QLIE native reader section budget"
        );
        Ok(out)
    }
}

/// Calculator object payloads are not length-prefixed. Their native class
/// serializer is the only legal boundary:6C4E50 ->6E6AA4 ->object.vmt+8.
/// A caller must supply the exact class codec; an unknown kind cannot be
/// skipped by scanning for a tag or by reporting a successful empty object.
pub(super) trait ObjectCodec {
    fn decode(
        &mut self,
        kind: &Text,
        identifier: &Text,
        reader: &mut Reader<'_>,
    ) -> Result<Vec<u8>>;
}
#[derive(Clone, Debug)]
pub(super) struct ObjectValue {
    pub identifier: Text,
    pub body: Vec<u8>,
}
#[derive(Clone, Debug)]
pub(super) enum Values {
    None,
    Integers {
        array_state: [u8; 16],
        values: Vec<i32>,
    },
    Strings {
        array_state: [u8; 16],
        values: Vec<Text>,
    },
    Objects {
        array_state: [u8; 16],
        values: Vec<ObjectValue>,
    },
}
///6C54AC scalar record before the typed value-array. The first three numbers
///retain native value/scope/index metadata. Array length follows its16-byte
///array state, not the variable's declaration dimensions in the VM.
#[derive(Clone, Debug)]
pub(super) struct ValueRecord {
    pub indices: [i32; 3],
    pub kind: u8,
    pub reference_only: u8,
    pub flags: i32,
    pub name: Text,
    pub source_kind: Text,
    pub values: Values,
}
impl ValueRecord {
    fn read(r: &mut Reader<'_>, objects: &mut dyn ObjectCodec) -> Result<Self> {
        let indices = r.ints(3)?.try_into().unwrap();
        let kind = r.byte()?;
        let reference_only = r.byte()?;
        let flags = r.int()?;
        let name = r.text()?;
        let source_kind = r.text()?;
        let values = if kind == 3 {
            Values::None
        } else {
            ensure!(
                kind <= 2,
                "unsupported QLIE native calculator value kind {kind}"
            );
            let array_state = r.bytes(16)?.try_into()?;
            let count = r.count(131072)?;
            match kind {
                0 => Values::Integers {
                    array_state,
                    values: r.ints(count)?,
                },
                1 => Values::Strings {
                    array_state,
                    values: r.texts(count)?,
                },
                2 => {
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        let identifier = r.text()?;
                        let body = if reference_only == 0 && !identifier.0.is_empty() {
                            objects.decode(&source_kind, &identifier, r)?
                        } else {
                            Vec::new()
                        };
                        values.push(ObjectValue { identifier, body });
                    }
                    Values::Objects {
                        array_state,
                        values,
                    }
                }
                _ => unreachable!(),
            }
        };
        Ok(Self {
            indices,
            kind,
            reference_only,
            flags,
            name,
            source_kind,
            values,
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        put_ints(out, &self.indices);
        out.push(self.kind);
        out.push(self.reference_only);
        put_int(out, self.flags);
        put_text(out, &self.name)?;
        put_text(out, &self.source_kind)?;
        match &self.values {
            Values::None => ensure!(self.kind == 3, "QLIE null value layout"),
            Values::Integers {
                array_state,
                values,
            } => {
                ensure!(self.kind == 0, "QLIE integer value layout");
                out.extend_from_slice(array_state);
                put_count(out, values.len())?;
                put_ints(out, values);
            }
            Values::Strings {
                array_state,
                values,
            } => {
                ensure!(self.kind == 1, "QLIE string value layout");
                out.extend_from_slice(array_state);
                put_count(out, values.len())?;
                put_texts(out, values)?;
            }
            Values::Objects {
                array_state,
                values,
            } => {
                ensure!(self.kind == 2, "QLIE object value layout");
                out.extend_from_slice(array_state);
                put_count(out, values.len())?;
                for value in values {
                    put_text(out, &value.identifier)?;
                    if self.reference_only == 0 && !value.identifier.0.is_empty() {
                        out.extend_from_slice(&value.body);
                    } else {
                        ensure!(
                            value.body.is_empty(),
                            "QLIE reference-only object carries payload"
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(super) struct Calculator {
    pub values: Vec<ValueRecord>,
    pub indices: Vec<Vec<ValueRecord>>,
}
impl Calculator {
    fn bank(r: &mut Reader<'_>, objects: &mut dyn ObjectCodec) -> Result<Vec<ValueRecord>> {
        let count = r.count(131072)?;
        (0..count).map(|_| ValueRecord::read(r, objects)).collect()
    }
    pub fn read(r: &mut Reader<'_>, objects: &mut dyn ObjectCodec) -> Result<Self> {
        ensure!(r.bytes(8)? == b"CalVal\0\0", "QLIE native calculator tag");
        ensure!(r.int()? == 1, "unsupported QLIE native calculator version");
        ensure!(
            r.int()? == 0,
            "unsupported QLIE native calculator reserved field"
        );
        let values = Self::bank(r, objects)?;
        ensure!(
            r.bytes(8)? == b"ValIdx\0\0",
            "QLIE native calculator index tag"
        );
        ensure!(
            r.int()? == 0,
            "unsupported QLIE native calculator index version"
        );
        let count = r.count(4096)?;
        let indices = (0..count)
            .map(|_| Self::bank(r, objects))
            .collect::<Result<_>>()?;
        Ok(Self { values, indices })
    }
    pub fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        out.extend_from_slice(b"CalVal\0\0");
        put_int(out, 1);
        put_int(out, 0);
        put_count(out, self.values.len())?;
        for value in &self.values {
            value.write(out)?;
        }
        out.extend_from_slice(b"ValIdx\0\0");
        put_int(out, 0);
        put_count(out, self.indices.len())?;
        for bank in &self.indices {
            put_count(out, bank.len())?;
            for value in bank {
                value.write(out)?;
            }
        }
        Ok(())
    }
}

/// 6A6620/6A62EC exact persistent camera state, including oscillators and
/// interpolation phase. Floating point words stay raw until runtime binding.
#[derive(Clone, Debug)]
pub(super) struct CameraTransform {
    pub axes: [i32; 27],
    pub scalars: [i32; 2],
    pub enabled: u8,
    pub colour: i32,
    pub projection: [i32; 9],
    pub matrices: [u8; 128],
    pub generators: [GeneratorRecord; 4],
    pub string_generators: StringGenerators,
    pub final_state: i32,
}
impl CameraTransform {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        ensure!(r.int()? == 3, "QLIE native camera transform version");
        let axes = r.ints(27)?.try_into().unwrap();
        let scalars = r.ints(2)?.try_into().unwrap();
        let enabled = r.byte()?;
        let colour = r.int()?;
        ensure!(r.int()? == 1, "QLIE native camera projection version");
        let projection = r.ints(9)?.try_into().unwrap();
        let matrices = r.bytes(128)?.try_into()?;
        let generators = (0..4)
            .map(|_| GeneratorRecord::read(r))
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .unwrap();
        let string_generators = StringGenerators::read(r)?;
        let final_state = r.int()?;
        Ok(Self {
            axes,
            scalars,
            enabled,
            colour,
            projection,
            matrices,
            generators,
            string_generators,
            final_state,
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        put_int(out, 3);
        put_ints(out, &self.axes);
        put_ints(out, &self.scalars);
        out.push(self.enabled);
        put_int(out, self.colour);
        put_int(out, 1);
        put_ints(out, &self.projection);
        out.extend_from_slice(&self.matrices);
        for generator in &self.generators {
            generator.write(out);
        }
        self.string_generators.write(out)?;
        put_int(out, self.final_state);
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(super) struct RetainedCamera {
    pub header: [u8; 36],
    pub state: [i32; 4],
    pub enabled: u8,
    pub transform: CameraTransform,
    pub selector: i32,
    pub planar: [i32; 7],
}
impl RetainedCamera {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let header: [u8; 36] = r.bytes(36)?.try_into()?;
        ensure!(
            i32::from_le_bytes(header[..4].try_into()?) == 10,
            "QLIE retained camera version"
        );
        let state = r.ints(4)?.try_into().unwrap();
        let enabled = r.byte()?;
        let transform = CameraTransform::read(r)?;
        let selector = r.int()?;
        ensure!(r.int()? == 2, "QLIE native planar camera version");
        let planar = r.ints(7)?.try_into().unwrap();
        Ok(Self {
            header,
            state,
            enabled,
            transform,
            selector,
            planar,
        })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        ensure!(
            i32::from_le_bytes(self.header[..4].try_into()?) == 10,
            "QLIE retained camera version"
        );
        out.extend_from_slice(&self.header);
        put_ints(out, &self.state);
        out.push(self.enabled);
        self.transform.write(out)?;
        put_int(out, self.selector);
        put_int(out, 2);
        put_ints(out, &self.planar);
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(super) struct TemporarySave {
    pub identifier: Text,
    /// Original eight timestamp/flag bytes, never a host memory pointer.
    pub state: [u8; 8],
    pub encoded: Vec<u8>,
}
#[derive(Clone, Debug)]
pub(super) struct TemporaryStore {
    pub header: [u8; 68],
    pub entries: Vec<TemporarySave>,
}
impl TemporaryStore {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let header: [u8; 68] = r.bytes(68)?.try_into()?;
        ensure!(
            i32::from_le_bytes(header[..4].try_into()?) == 10,
            "QLIE native temporary store version"
        );
        let count = r.count(65536)?;
        let entries = (0..count)
            .map(|_| {
                let identifier = r.text()?;
                let state = r.bytes(8)?.try_into()?;
                let count = r.count(LIMIT as usize)?;
                let encoded = r.bytes(count)?.to_vec();
                Ok(TemporarySave {
                    identifier,
                    state,
                    encoded,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self { header, entries })
    }
    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        out.extend_from_slice(&self.header);
        put_count(out, self.entries.len())?;
        for entry in &self.entries {
            put_text(out, &entry.identifier)?;
            out.extend_from_slice(&entry.state);
            put_count(out, entry.encoded.len())?;
            out.extend_from_slice(&entry.encoded);
        }
        Ok(())
    }
}
/// Application-specific callbacks precede the fixed419-byte script block.
/// Their length is not present in the stream: a known callback codec must
/// consume it. Searching for the next version word is never a safe substitute.
pub(super) trait CallbackCodec {
    fn decode(&mut self, reader: &mut Reader<'_>) -> Result<Vec<u8>>;
}
/// Complete6D5218/6D4D24 body after decrypt/decompress. The object and optional
/// application callback codecs own their original unframed payload boundaries.
#[derive(Clone, Debug)]
pub(super) struct Body {
    pub reader: ReaderSections,
    pub calculator: Calculator,
    pub callback: Vec<u8>,
    pub script_state: [u8; 419],
    pub labels: [Text; 7],
    pub label_log: [u8; 69],
    pub file_log_header: [u8; 69],
    pub files: Vec<Text>,
    pub temporary: TemporaryStore,
    pub input: [u8; 264],
    pub camera: RetainedCamera,
    pub metadata: [Text; 8],
}
impl Body {
    pub fn decode(
        bytes: &[u8],
        objects: &mut dyn ObjectCodec,
        callback: Option<&mut dyn CallbackCodec>,
    ) -> Result<Self> {
        let reader = ReaderSections::decode(bytes)?;
        let mut r = Reader::new(bytes)?;
        r.position = reader.consumed;
        let calculator = Calculator::read(&mut r, objects)?;
        let callback = callback
            .map(|codec| codec.decode(&mut r))
            .transpose()?
            .unwrap_or_default();
        let script_state: [u8; 419] = r.bytes(419)?.try_into()?;
        ensure!(
            i32::from_le_bytes(script_state[..4].try_into()?) == 10,
            "QLIE native script state version"
        );
        let labels = r.texts(7)?.try_into().unwrap();
        let label_log: [u8; 69] = r.bytes(69)?.try_into()?;
        ensure!(
            i32::from_le_bytes(label_log[..4].try_into()?) == 10,
            "QLIE native label log version"
        );
        let file_log_header: [u8; 69] = r.bytes(69)?.try_into()?;
        ensure!(
            i32::from_le_bytes(file_log_header[..4].try_into()?) == 10,
            "QLIE native file log version"
        );
        let count = r.count(65536)?;
        let files = r.texts(count)?;
        let temporary = TemporaryStore::read(&mut r)?;
        let input: [u8; 264] = r.bytes(264)?.try_into()?;
        ensure!(&input[..8] == b"imotwt\0\0", "QLIE native input state tag");
        let camera = RetainedCamera::read(&mut r)?;
        let metadata = r.texts(8)?.try_into().unwrap();
        ensure!(r.remaining().is_empty(), "QLIE native body trailing bytes");
        Ok(Self {
            reader,
            calculator,
            callback,
            script_state,
            labels,
            label_log,
            file_log_header,
            files,
            temporary,
            input,
            camera,
            metadata,
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = self.reader.encode()?;
        self.calculator.write(&mut out)?;
        out.extend_from_slice(&self.callback);
        out.extend_from_slice(&self.script_state);
        put_texts(&mut out, &self.labels)?;
        out.extend_from_slice(&self.label_log);
        out.extend_from_slice(&self.file_log_header);
        put_count(&mut out, self.files.len())?;
        put_texts(&mut out, &self.files)?;
        self.temporary.write(&mut out)?;
        out.extend_from_slice(&self.input);
        self.camera.write(&mut out)?;
        put_texts(&mut out, &self.metadata)?;
        ensure!(
            out.len() as u64 <= LIMIT,
            "QLIE native full-body byte budget"
        );
        Ok(out)
    }
}

#[derive(Clone, Debug)]
pub(super) struct TextLayout {
    pub geometry: [i32; 4],
    pub glyphs: Vec<[u8; 20]>,
    pub strings: Vec<(Text, i32)>,
}
impl TextLayout {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        ensure!(r.int()? == 1, "QLIE native text layout version");
        let geometry = r.ints(4)?.try_into().unwrap();
        let count = r.count(1_000_000)?;
        let glyphs = (0..count)
            .map(|_| Ok(r.bytes(20)?.try_into()?))
            .collect::<Result<_>>()?;
        let count = r.count(65536)?;
        let strings = (0..count)
            .map(|_| Ok((r.text()?, r.int()?)))
            .collect::<Result<_>>()?;
        Ok(Self {
            geometry,
            glyphs,
            strings,
        })
    }
}
#[derive(Clone, Debug)]
pub(super) struct MessageObject {
    pub image: Image,
    pub state: [u8; 189],
    pub strings: [Text; 5],
    pub layout: TextLayout,
}
impl MessageObject {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let image = Image::read(r)?;
        let state: [u8; 189] = r.bytes(189)?.try_into()?;
        ensure!(
            i32::from_le_bytes(state[..4].try_into()?) == 12,
            "QLIE native message version"
        );
        let strings = r.texts(5)?.try_into().unwrap();
        let layout = TextLayout::read(r)?;
        Ok(Self {
            image,
            state,
            strings,
            layout,
        })
    }
}
#[derive(Clone, Debug)]
pub(super) enum ObjectPayload {
    Empty,
    Image(Image),
    Message(MessageObject),
    Button {
        message: MessageObject,
        state: [u8; 42],
    },
    Scrollbar {
        state: [u8; 187],
        strings: [Text; 3],
        values: Vec<(i32, Text)>,
        images: [Image; 3],
    },
    File(Text),
    Strings(Vec<Text>),
    Theora {
        state: [u8; 131],
        resource: Text,
    },
    Timer {
        state: [u8; 89],
        callback: [Text; 2],
    },
}
#[derive(Clone, Debug)]
pub(super) struct NativeObject {
    pub kind: String,
    pub flags: i32,
    pub payload: ObjectPayload,
}
impl NativeObject {
    pub fn read(kind: &Text, r: &mut Reader<'_>) -> Result<Self> {
        let kind = String::from_utf16(&kind.0)?.to_ascii_lowercase();
        // 6E6AA4 invokes the object's +8 SaveToStream; +16 is a
        // prepare-save callback, not the payload serializer itself.
        ensure!(
            r.bytes(4)? == b"objd" && r.int()? == 1,
            "QLIE native object header"
        );
        let flags = r.int()?;
        let payload = match kind.as_str() {
            "img" | "simg" => ObjectPayload::Image(Image::read(r)?),
            "msg" => ObjectPayload::Message(MessageObject::read(r)?),
            "btn" | "sbtn" => {
                let message = MessageObject::read(r)?;
                let state: [u8; 42] = r.bytes(42)?.try_into()?;
                ensure!(
                    i32::from_le_bytes(state[..4].try_into()?) == 10,
                    "QLIE native button version"
                );
                ObjectPayload::Button { message, state }
            }
            "bar" => {
                let state: [u8; 187] = r.bytes(187)?.try_into()?;
                ensure!(
                    i32::from_le_bytes(state[..4].try_into()?) == 10,
                    "QLIE native scrollbar version"
                );
                let strings = r.texts(3)?.try_into().unwrap();
                let values = (0..8)
                    .map(|_| Ok((r.int()?, r.text()?)))
                    .collect::<Result<_>>()?;
                let images = (0..3)
                    .map(|_| Image::read(r))
                    .collect::<Result<Vec<_>>>()?
                    .try_into()
                    .unwrap();
                ObjectPayload::Scrollbar {
                    state,
                    strings,
                    values,
                    images,
                }
            }
            "file" => ObjectPayload::File(r.text()?),
            "list" | "table" | "ppngpool" => {
                let count = r.count(65536)?;
                ObjectPayload::Strings(r.texts(count)?)
            }
            "theora" => {
                let state: [u8; 131] = r.bytes(131)?.try_into()?;
                ensure!(
                    i32::from_le_bytes(state[..4].try_into()?) == 10,
                    "QLIE native movie version"
                );
                ObjectPayload::Theora {
                    state,
                    resource: r.text()?,
                }
            }
            "timer" => {
                let state: [u8; 89] = r.bytes(89)?.try_into()?;
                ensure!(
                    i32::from_le_bytes(state[..4].try_into()?) == 10,
                    "QLIE native timer version"
                );
                ObjectPayload::Timer {
                    state,
                    callback: r.texts(2)?.try_into().unwrap(),
                }
            }
            // Native wrapper classes below inherit6B2118 exactly. Their
            // backing managers are serialized earlier in the reader stream.
            "sys" | "dat" | "sdat" | "snd" | "dsnd" | "eff" | "scsr" | "csr" | "log" | "rep"
            | "path" | "slog" | "flog" | "llog" | "math" | "cfg" | "misc" | "inp" | "skey"
            | "clip" | "dlgs" | "reg" | "mdat" | "tdat" | "cam" | "camex" | "twitter" | "ext" => {
                ObjectPayload::Empty
            }
            _ => bail!("unsupported QLIE native object payload class {kind}"),
        };
        Ok(Self {
            kind,
            flags,
            payload,
        })
    }
}
/// Verified built-in class codec; captures raw bytes only after the typed
/// parser has established the exact unframed record boundary.
#[derive(Default)]
pub(super) struct BuiltinObjectCodec;
impl ObjectCodec for BuiltinObjectCodec {
    fn decode(&mut self, kind: &Text, _identifier: &Text, r: &mut Reader<'_>) -> Result<Vec<u8>> {
        let start = r.position;
        let _object = NativeObject::read(kind, r)?;
        Ok(r.source[start..r.position].to_vec())
    }
}
