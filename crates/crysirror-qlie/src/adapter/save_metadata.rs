//! Original IMOSAVE110 envelope metadata, not a substitute VM snapshot format.
//! Native 6B6AA4/6B6F78: checksum envelope, 138-byte header, eight UTF-16
//! strings, length-prefixed JPEG and encoded VM body. The metadata query never
//! decompresses or restores that body (native dat\g uses a5=0).
use super::*;
use crysirror_core::service::ClientStorage;
use crysirror_resource::lane_feedback;

pub(super) const LIMIT: u64 = 64 * 1024 * 1024;
pub(super) struct Metadata {
    date: f64,
    strings: [String; 8],
}
pub(super) enum ReadResult {
    Missing,
    Invalid(String),
    Ready(Metadata),
}

fn take<'a>(source: &mut &'a [u8], len: usize) -> Result<&'a [u8]> {
    ensure!(source.len() >= len, "truncated QLIE save envelope");
    let (head, rest) = source.split_at(len);
    *source = rest;
    Ok(head)
}
fn length(source: &mut &[u8]) -> Result<usize> {
    let n = i32::from_le_bytes(take(source, 4)?.try_into().unwrap());
    usize::try_from(n).context("negative QLIE save block length")
}
impl Metadata {
    fn parse(mut bytes: &[u8], key: u32) -> Result<Self> {
        // The native stream wrapper (4E6468/4E6514) validates the checksum
        // before handing the bounded payload to TImoSaveData.
        ensure!(
            take(&mut bytes, 4)? == 0xfd324d4eu32.to_le_bytes(),
            "QLIE save envelope tag"
        );
        let checksum = u32::from_le_bytes(take(&mut bytes, 4)?.try_into().unwrap());
        let size = length(&mut bytes)?;
        let mut body = take(&mut bytes, size)?;
        ensure!(
            lane_feedback::checksum(body) == checksum,
            "QLIE save envelope checksum"
        );
        let header = take(&mut body, 138)?;
        ensure!(
            header[..16].split(|b| *b == 0).next() == Some(b"IMOSAVE110"),
            "QLIE save signature"
        );
        let mut tag: [u8; 40] = header[16..56].try_into().unwrap();
        lane_feedback::decrypt(&mut tag, key);
        ensure!(
            tag[..32] == *b"we8wq8q986sfaifg8try695hj5iukt5w" && tag[32..].iter().all(|b| *b == 0),
            "QLIE save key verification"
        );
        let version = i32::from_le_bytes(header[56..60].try_into().unwrap());
        ensure!(
            (0..=11).contains(&version),
            "unsupported QLIE save version {version}"
        );
        let date = f64::from_le_bytes(header[62..70].try_into().unwrap());
        let mut strings = std::array::from_fn(|_| String::new());
        for string in &mut strings {
            let count = i16::from_le_bytes(take(&mut body, 2)?.try_into().unwrap());
            let count = usize::try_from(count).context("negative QLIE save string length")?;
            let units = take(&mut body, count * 2)?
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]))
                .collect::<Vec<_>>();
            *string = String::from_utf16(&units).context("QLIE save metadata UTF16")?;
        }
        let image_size = length(&mut body)?;
        take(&mut body, image_size)?;
        let data_size = length(&mut body)?;
        take(&mut body, data_size)?;
        Ok(Self { date, strings })
    }
    fn calendar(&self) -> Result<[i32; 7]> {
        // Delphi TDateTime: signed days since1899-12-30; negative dates use
        // an absolute fractional time. Native yyyy/mm/dd/hh/nn/ss/zzz queries
        // read local civil fields, without applying the host's timezone.
        ensure!(self.date.is_finite(), "nonfinite QLIE save date");
        let days = self.date.trunc();
        ensure!(
            (-693593. ..=2958465.).contains(&days),
            "QLIE save calendar range"
        );
        let (year, month, day) =
            crysirror_core::service::normalize_civil_date(1899, 12, 30 + days as i32)
                .context("QLIE save calendar conversion")?;
        let ms = (self.date.fract().abs() * 86400000.).round_ties_even() as u64 % 86400000;
        Ok([
            year,
            i32::from(month),
            i32::from(day),
            (ms / 3600000) as i32,
            (ms / 60000 % 60) as i32,
            (ms / 1000 % 60) as i32,
            (ms % 1000) as i32,
        ])
    }
}
pub(super) fn read(
    storage: Arc<dyn ClientStorage>,
    name: &str,
    key: u32,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<ReadResult> {
    cancel.check()?;
    let Some(info) = storage.metadata(Mount::Save, name)? else {
        return Ok(ReadResult::Missing);
    };
    ensure!(
        !info.is_directory && info.size.is_none_or(|size| size <= LIMIT),
        "QLIE original save byte budget/type"
    );
    let reader = storage.open_read(&VfsPath::new(Mount::Save, name)?)?;
    let data = crysirror_resource::read_all(reader.as_ref(), LIMIT)?;
    cancel.check()?;
    Ok(match Metadata::parse(&data, key) {
        Ok(meta) => ReadResult::Ready(meta),
        Err(error) => ReadResult::Invalid(format!("{error:#}")),
    })
}
impl QlieVm {
    pub(super) fn query_save(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        name: &str,
    ) -> Result<bool> {
        let name = crate::program::canonical(name)?;
        if self.save_query.is_none() {
            let storage = h.storage(self.client)?;
            // Native5054F4 defaults CodeKey to81789, masks it to24 bits.
            let key = self
                .text
                .get("system/etc/game.txt")
                .into_iter()
                .flat_map(|s| s.lines())
                .filter_map(|line| line.split_once('='))
                .find(|(key, _)| key.trim().eq_ignore_ascii_case("CodeKey"))
                .and_then(|(_, value)| crate::expression::parse_integer(value))
                .unwrap_or(81789) as u32
                & 0xffffff;
            let source = name.clone();
            self.save_query = Some((
                name.clone(),
                self.pool
                    .spawn(move |cancel| read(storage, &source, key, &cancel))?,
            ));
        }
        let (expected, task) = self.save_query.as_mut().unwrap();
        ensure!(*expected == name, "QLIE save metadata completion owner");
        let Some(result) = task.try_take()? else {
            return Ok(false);
        };
        self.save_query = None;
        for n in 0..64 {
            m.result(n, Value::Int(0));
            m.result_text(n, String::new());
        }
        match result {
            ReadResult::Missing => (),
            ReadResult::Invalid(error) => {
                // A corrupt/incompatible slot is the native failed lookup,
                // rather than an acknowledged restore or a usable save.
                h.append_log(self.client, &format!("QLIE save metadata {name}: {error}"))?;
            }
            ReadResult::Ready(meta) => {
                for (n, value) in meta.calendar()?.into_iter().enumerate() {
                    m.result(n, Value::Int(value));
                }
                for (n, value) in meta.strings.into_iter().enumerate() {
                    m.result_text(n, value);
                }
            }
        }
        Ok(true)
    }
}
