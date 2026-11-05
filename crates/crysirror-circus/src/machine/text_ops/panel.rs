//! Native437520/437780/4377D0/435880/435920 declarations. These are original
//! resource/state records; shared Ui owns layout, hit testing, drag and focus.
use super::*;
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelConfig {
    pub geometry: [i16; 8],
    pub sounds: [u16; 2],
    pub voices: [String; 2],
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelText {
    pub position: [i16; 2],
    pub absolute: bool,
    pub variants: BTreeMap<i16, (i16, String)>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelRow {
    pub slot_id: i32,
    pub image: Option<ImagePart>,
    pub states: Vec<ImagePart>,
    pub texts: Vec<PanelText>,
    pub action: String,
    pub(crate) declared_image: bool,
    ///435920's three2372-byte text records, followed by the32-byte action.
    ///strcpy overwrites only through its byte NUL; UTF16 views retain old tails.
    ///The final unknown DWORD and preceding image/pointer header are excluded.
    #[serde(default)]
    pub(crate) native_text_bytes: Vec<u8>,
}
const TEXT_STRIDE:usize=2372;
const TEXT_BYTES:usize=TEXT_STRIDE*3;
const ARENA_BYTES:usize=TEXT_BYTES+32;
///Host-generated/normalized text has no original MES byte provenance. CP932
///has duplicate encodings (notably IBM/NEC extensions); reject such ambiguity
///before a byte alias reaches UTF16. Parsed literals use the original sidecar.
fn ensure_unique_cp932_literal(text:&str)->Result<()> {
    static AMBIGUOUS:std::sync::OnceLock<BTreeSet<char>>=std::sync::OnceLock::new();
    let ambiguous=AMBIGUOUS.get_or_init(||{
        let mut result=BTreeSet::new();
        for lead in (0x81u8..=0x9f).chain(0xe0..=0xfc) {
            for trail in (0x40u8..=0x7e).chain(0x80..=0xfc) {
                let pair=[lead,trail];
                let (decoded,_,error)=encoding_rs::SHIFT_JIS.decode(&pair);
                if error{continue}
                let mut chars=decoded.chars();
                let Some(ch)=chars.next()else{continue};
                if chars.next().is_some(){continue}
                let (canonical,_,error)=encoding_rs::SHIFT_JIS.encode(&decoded);
                if !error&&canonical.as_ref()!=pair {result.insert(ch);}
            }
        }
        result
    });
    ensure!(!text.chars().any(|ch|ambiguous.contains(&ch)),"Circus panel wide byte alias needs original CP932 literal bytes: normalized host/generated text has ambiguous byte provenance");
    Ok(())
}
impl PanelRow {
    fn text_arena(&self)->Result<Vec<u8>> {
        if !self.native_text_bytes.is_empty(){
            ensure!(self.native_text_bytes.len()==ARENA_BYTES,"Circus panel native text arena extent");
            return Ok(self.native_text_bytes.clone());
        }
        let mut bytes=vec![0;ARENA_BYTES];
        ensure!(self.texts.len()<=3,"Circus panel text record count");
        for (line,text) in self.texts.iter().enumerate(){
            let base=line*TEXT_STRIDE;
            for (offset,value) in [(0,i32::from(text.position[0])),(4,i32::from(text.position[1]))]{bytes[base+offset..base+offset+4].copy_from_slice(&value.to_le_bytes());}
            for (&state,(color,value)) in &text.variants {
                let state=if state==99{0}else{usize::try_from(state)?};
                ensure!(state<3,"Circus legacy panel text state");
                bytes[base+8+state*4..base+12+state*4].copy_from_slice(&i32::from(*color).to_le_bytes());
                bytes[base+20+state*4..base+24+state*4].copy_from_slice(&i32::from(text.absolute).to_le_bytes());
                let (value,_,error)=encoding_rs::SHIFT_JIS.encode(value);
                ensure!(!error&&value.len()<260,"Circus legacy panel CP932 text extent");
                let at=base+32+state*260;bytes[at..at+value.len()].copy_from_slice(&value);
            }
        }
        let (action,_,error)=encoding_rs::SHIFT_JIS.encode(&self.action);
        ensure!(!error&&action.len()<32,"Circus panel action encoding");
        bytes[TEXT_BYTES..TEXT_BYTES+action.len()].copy_from_slice(&action);
        Ok(bytes)
    }
    ///Decode the native record's actual narrow/wide view only at consumption.
    ///The producer remains a CP932-byte strcpy, never a Unicode wide setter.
    pub(crate) fn rendered_texts(&self)->Result<Vec<PanelText>> {
        if self.native_text_bytes.is_empty(){
            return Ok(self.texts.iter().cloned().map(|mut t|{t.variants.retain(|_,(color,_)|*color>0);t}).collect());
        }
        let bytes=&self.native_text_bytes;
        ensure!(bytes.len()==ARENA_BYTES,"Circus panel native text arena extent");
        let word=|at:usize|i32::from_le_bytes(bytes[at..at+4].try_into().unwrap());
        let mut result=Vec::new();
        for line in 0..3 {
            let base=line*TEXT_STRIDE;
            let wide=u16::from_le_bytes(bytes[base+812..base+814].try_into()?)!=0;
            let variants=word(base+12)!=0||word(base+16)!=0;
            for state in 0..if variants{3}else{1} {
                let color=word(base+8+state*4);
                if variants&&color<=0{continue}
                let at=base+if wide{812+state*520}else{32+state*260};
                let text=if wide {
                    let mut units=Vec::new();let mut cursor=at;
                    loop {
                        ensure!(cursor+2<=bytes.len(),"Circus panel unterminated native UTF16 view");
                        let unit=u16::from_le_bytes(bytes[cursor..cursor+2].try_into()?);cursor+=2;
                        if unit==0{break}units.push(unit);
                    }
                    String::from_utf16(&units).context("Circus panel UTF16 view contains unsupported unpaired surrogate")?
                }else{
                    let end=bytes[at..].iter().position(|v|*v==0).context("Circus panel unterminated native CP932 view")?+at;
                    let (text,_,error)=encoding_rs::SHIFT_JIS.decode(&bytes[at..end]);
                    ensure!(!error,"Circus panel aliased CP932 view is malformed");text.into_owned()
                };
                if text.is_empty(){continue}
                result.push(PanelText{
                    position:[i16::try_from(word(base))?,i16::try_from(word(base+4))?],
                    absolute:variants&&word(base+20+state*4)!=0,
                    variants:[(if state==0{99}else{state as i16},(i16::try_from(color).context("Circus panel aliased text color exceeds native adapter color extent")?,text))].into(),
                });
            }
        }
        Ok(result)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelDeclarations {
    pub configs: Vec<PanelConfig>,
    pub rectangles: Vec<[i16; 4]>,
    pub overlays: Vec<ImagePart>,
    pub thumb: Option<ImagePart>,
    pub rows: Vec<PanelRow>,
    pub step: i16,
    pub current_config: Option<PanelConfig>,
    pub current_row: Option<PanelRow>,
    pub text_color: i16,
}
impl Default for PanelDeclarations {
    fn default() -> Self {
        Self {
            configs: vec![],
            rectangles: vec![],
            overlays: vec![],
            thumb: None,
            rows: vec![],
            step: 1,
            current_config: None,
            current_row: None,
            text_color: 0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct PanelReply {
    pub scroll_changed: bool,
    pub scroll: i32,
    pub focus: i32,
    pub slot_id: Option<i32>,
    pub action: Option<String>,
}
impl PanelDeclarations {
    pub fn has_viewport(&self) -> bool {
        self.current_config
            .as_ref()
            .or(self.configs.last())
            .is_some_and(|config| {
                config.geometry[4] != config.geometry[6] || config.geometry[5] != config.geometry[7]
            })
    }
    pub fn config(&mut self, a: [i16; 8]) -> Result<()> {
        if let Some(config) = self.current_config.take() {
            ensure!(self.configs.len() < 64, "Circus panel config budget");
            self.configs.push(config);
        }
        self.current_config = Some(PanelConfig {
            geometry: a,
            ..Default::default()
        });
        Ok(())
    }
    pub fn begin_row(&mut self, a: [i16; 8]) -> Result<()> {
        if let Some(row) = self.current_row.take().filter(|row| row.declared_image) {
            ensure!(self.rows.len() < 4096, "Circus panel row budget");
            self.rows.push(row);
        }
        let slot = u8::try_from(a[6]).context("Circus panel row image slot")?;
        ensure!(slot < 64, "Circus panel row image slot range");
        self.current_row = Some(PanelRow {
            slot_id: i32::from(a[0]),
            image: Some(ImagePart {
                position: [0, 0],
                source: [
                    a[2],
                    a[3],
                    a[4].checked_sub(a[2])
                        .context("Circus panel source x overflow")?,
                    a[5].checked_sub(a[3])
                        .context("Circus panel source y overflow")?,
                ],
                slot,
                state: 99,
                auxiliary_flag: false,
                resource_binding:None,binding_declared:false,
            }),
            declared_image: true,
            ..Default::default()
        });
        Ok(())
    }
    pub fn state_image(&mut self, part: ImagePart) {
        let row = self.current_row.get_or_insert_with(PanelRow::default);
        row.states.retain(|old| old.state != part.state);
        row.states.push(part);
    }
    pub fn action(&mut self, text: &str) -> Result<()> {
        let (encoded, _, error) = encoding_rs::SHIFT_JIS.encode(text);
        ensure!(
            !error && encoded.len() < 32,
            "Circus panel row action extent/encoding"
        );
        let row=self.current_row.get_or_insert_with(PanelRow::default);
        if !row.native_text_bytes.is_empty(){
            row.native_text_bytes[TEXT_BYTES..TEXT_BYTES+encoded.len()].copy_from_slice(&encoded);
            row.native_text_bytes[TEXT_BYTES+encoded.len()]=0;
        }
        row.action = text.into();
        Ok(())
    }
    pub fn text(&mut self, a: [i16; 8], text: &str) -> Result<()> {
        let (encoded, _, error) = encoding_rs::SHIFT_JIS.encode(text);
        ensure!(!error,"Circus panel literal has no CP932 byte representation");
        self.write_text(a,text,&encoded,false)
    }
    ///Only the parent VM dispatcher may supply parser-owned original bytes.
    pub(super) fn text_bytes(&mut self,a:[i16;8],text:&str,raw:&[u8])->Result<()> {
        let (decoded,_,error)=encoding_rs::SHIFT_JIS.decode(raw);
        ensure!(!error&&decoded==text,"Circus panel raw literal does not match its parser-owned decoded instruction");
        //The native strcpy terminates at the first byte NUL even if a decoded
        //encrypted instruction retains later bytes in its parser sidecar.
        let end=raw.iter().position(|b|*b==0).unwrap_or(raw.len());
        self.write_text(a,text,&raw[..end],true)
    }
    fn write_text(&mut self,a:[i16;8],text:&str,encoded:&[u8],original_bytes:bool)->Result<()> {
        let row = self.current_row.get_or_insert_with(PanelRow::default);
        let index=usize::try_from(a[3]).context("Circus panel negative text variant aliases unknown image/header fields")?;
        let at=32usize.checked_add(index.checked_mul(260).context("Circus panel text alias overflow")?).context("Circus panel text alias overflow")?;
        let color=8+index*4;let absolute=20+index*4;
        ensure!(at+encoded.len()+1<=ARENA_BYTES&&color+4<=ARENA_BYTES&&absolute+4<=ARENA_BYTES,"Circus panel text alias exceeds known text/action arena (unknown trailing field or object boundary)");
        if !original_bytes&&at+encoded.len()>=812 {ensure_unique_cp932_literal(text)?;}
        let mut bytes=row.text_arena()?;
        bytes[..4].copy_from_slice(&i32::from(a[0]).to_le_bytes());
        bytes[4..8].copy_from_slice(&i32::from(a[1]).to_le_bytes());
        bytes[color..color+4].copy_from_slice(&i32::from(self.text_color).to_le_bytes());
        bytes[absolute..absolute+4].copy_from_slice(&i32::from(a[2]).to_le_bytes());
        bytes[at..at+encoded.len()].copy_from_slice(encoded);bytes[at+encoded.len()]=0;
        let end=bytes[TEXT_BYTES..].iter().position(|v|*v==0).context("Circus panel text alias removed action terminator")?+TEXT_BYTES;
        let (action,_,error)=encoding_rs::SHIFT_JIS.decode(&bytes[TEXT_BYTES..end]);
        ensure!(!error,"Circus panel text alias produced malformed action CP932");
        row.action=action.into_owned();row.native_text_bytes=bytes;
        //Legacy introspection only. Rendering consumes the raw alias-preserving
        //arena above, including color/absolute writes into neighboring views.
        if index<3 {
            if row.texts.is_empty(){row.texts.push(PanelText::default());}
            let part=&mut row.texts[0];part.position=[a[0],a[1]];part.absolute=a[2]!=0;
            part.variants.insert(if index==0{99}else{index as i16},(self.text_color,text.into()));
        }
        Ok(())
    }
    pub fn finalized(&self) -> Result<Self> {
        let mut result = self.clone();
        //4375E0 memcpy's the live declaration buffers without clearing
        //them. A later finalize or437520/4377D0 therefore copies them again.
        if let Some(config) = result.current_config.clone() {
            ensure!(
                result.configs.len() < 64,
                "Circus panel configuration budget"
            );
            result.configs.push(config);
        }
        if let Some(row) = result
            .current_row
            .clone()
            .filter(|row| !row.action.is_empty())
        {
            ensure!(result.rows.len() < 4096, "Circus panel row budget");
            result.rows.push(row);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn row_commit_keeps_native_id_states_and_per_state_colors() {
        let mut panel = PanelDeclarations::default();
        panel.begin_row([17, 0, 3, 4, 23, 34, 19, 0]).unwrap();
        panel.action("activate").unwrap();
        panel.text_color = 7;
        panel.text([1, 2, 0, 0, 0, 0, 0, 0], "normal").unwrap();
        panel.text_color = 129;
        panel.text([1, 2, 1, 2, 0, 0, 0, 0], "selected").unwrap();
        let done = panel.finalized().unwrap();
        assert_eq!(done.rows.len(), 1);
        assert_eq!(done.rows[0].slot_id, 17);
        assert_eq!(done.rows[0].image.as_ref().unwrap().source, [3, 4, 20, 30]);
        assert_eq!(done.rows[0].texts[0].variants[&99], (7, "normal".into()));
        assert_eq!(done.rows[0].texts[0].variants[&2], (129, "selected".into()));
        assert!(
            panel.rows.is_empty(),
            "preparation must not consume original builder on retry"
        );
    }
}
