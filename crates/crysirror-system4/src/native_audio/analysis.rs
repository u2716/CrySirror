// GPL-2.0-or-later. Original SACT2 10700/10740/10C20 and MusicSystem
// 2060/2120 inspected completely; shared audio owns PCM/FFT/window kernels.
use super::*;
use crate::vm::heap::PageKind;
use crysirror_core::service::{AudioSampleQuery, AudioSampleRequest, AudioSampleState};
pub(super) struct SampleRead {
    call: ExternalCall,
    channel: AudioResourceHandle,
    job: Box<dyn AudioSampleQuery>,
}
fn array(m: &Machine, slot: i32, kind: i32) -> Result<Vec<i32>> {
    let page = m.heap.page(slot)?;
    ensure!(
        matches!(page.kind,PageKind::Array{kind:k,rank:1,..} if k==kind),
        "native analysis rank1 array ABI"
    );
    ensure!(page.values.len() <= 65536, "native analysis array budget");
    Ok(page.values.clone())
}
impl NativeAudio {
    pub(super) fn dispatch_analysis(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            return Ok(None);
        };
        let program = m.program.clone();
        let lib = &program.libraries[library];
        let function = &lib.functions[function];
        let name = function.name.as_slice();
        if lib.name != b"SACT2"
            || !matches!(
                name,
                b"FFT_rdft"
                    | b"FFT_hanning_window"
                    | b"Music_AnalyzeSampleData"
                    | b"Music_GetSampleData"
                    | b"Music_GetFormat"
            )
        {
            ensure!(
                self.sample_read.is_none(),
                "native sample call changed while pending"
            );
            return Ok(None);
        }
        ensure!(
            owner == self.owner && bridge.sact.as_ref().is_some_and(|s| s.initialized),
            "native audio analysis owner/init"
        );
        let args = m.hll_arguments(call)?;
        if matches!(name, b"FFT_rdft" | b"FFT_hanning_window") {
            ensure!(
                function.return_kind == 0 && self.sample_read.is_none(),
                "native FFT return/pending ABI"
            );
            let [Argument::HeapReference { kind: 23, slot }] = args.as_slice() else {
                anyhow::bail!("native FFT float array ABI")
            };
            let input = array(m, *slot, 15)?
                .into_iter()
                .map(|n| f32::from_bits(n as u32))
                .collect::<Vec<_>>();
            let output = if name == b"FFT_rdft" {
                crysirror_audio::analysis::real_spectrum(&input, &|| Ok(()))?
            } else {
                self.hann
                    .apply(&input, f32::from_bits(1086918619), &|| Ok(()))?
            };
            let values = output
                .into_iter()
                .map(|n| n.to_bits() as i32)
                .collect::<Vec<_>>();
            m.heap.page_mut(*slot)?.values.copy_from_slice(&values);
            m.complete_hll(call, HllReturn::Void)?;
            return Ok(Some(Dispatch::Completed));
        }
        if name == b"Music_AnalyzeSampleData" {
            ensure!(
                function.return_kind == 10 && self.sample_read.is_none(),
                "native PCM analysis return/pending ABI"
            );
            let [Argument::HeapReference { kind: 23, slot: l }, Argument::HeapReference { kind: 23, slot: r }, Argument::HeapReference {
                kind: 22,
                slot: source,
            }, Argument::Scalar {
                kind: 10,
                bits: channels,
            }, Argument::Scalar { kind: 10, bits }] = args.as_slice()
            else {
                anyhow::bail!("native PCM analysis ABI")
            };
            let source = array(m, *source, 14)?;
            let left = array(m, *l, 15)?;
            let right = array(m, *r, 15)?;
            if source.is_empty() || !matches!(channels, 1 | 2) || !matches!(bits, 8 | 16) {
                m.complete_hll(call, HllReturn::Scalar(0))?;
                return Ok(Some(Dispatch::Completed));
            }
            let (out_l, out_r) = crysirror_audio::analysis::split_pcm_words(
                &source,
                *channels as u8,
                *bits as u8,
                &|| Ok(()),
            )?;
            ensure!(
                left.len() >= out_l.len() && out_r.as_ref().is_none_or(|v| right.len() >= v.len()),
                "native PCM analysis output capacity"
            );
            for (i, value) in out_l.iter().enumerate() {
                m.heap.page_mut(*l)?.values[i] = value.to_bits() as i32;
            }
            if let Some(values) = out_r {
                for (i, value) in values.iter().enumerate() {
                    m.heap.page_mut(*r)?.values[i] = value.to_bits() as i32;
                }
            }
            m.complete_hll(call, HllReturn::Scalar(1))?;
            return Ok(Some(Dispatch::Completed));
        }
        ensure!(
            function.return_kind == 10,
            "native music sample query return ABI"
        );
        let (number, max_bytes, start) = match (name, args.as_slice()) {
            (
                b"Music_GetSampleData",
                [Argument::Scalar {
                    kind: 10,
                    bits: number,
                }, Argument::HeapReference { kind: 22, slot }, Argument::Scalar {
                    kind: 10,
                    bits: start,
                }],
            ) => {
                let size = array(m, *slot, 14)?.len() * 4;
                ensure!(*start >= 0, "native music negative sample position");
                (*number, size as u32, *start as u64)
            }
            (
                b"Music_GetFormat",
                [Argument::Scalar {
                    kind: 10,
                    bits: number,
                }, rest @ ..],
            ) => {
                ensure!(rest.len() == 3, "native music format ref count");
                for arg in rest {
                    let Argument::Reference {
                        kind: 18,
                        page,
                        index,
                    } = arg
                    else {
                        anyhow::bail!("native music format ref ABI")
                    };
                    m.heap.get(*page, *index)?;
                }
                (*number, 0, 0)
            }
            _ => anyhow::bail!("native music sample query ABI"),
        };
        let Some(channel) = self.channels.get(&Key {
            kind: Kind::Music,
            number,
        }) else {
            ensure!(
                self.sample_read.is_none(),
                "native music sample channel vanished"
            );
            m.complete_hll(call, HllReturn::Scalar(0))?;
            return Ok(Some(Dispatch::Completed));
        };
        if self.sample_read.is_none() {
            self.sample_read = Some(SampleRead {
                call: call.clone(),
                channel: channel.resource,
                job: host.begin_audio_samples(
                    owner,
                    channel.resource,
                    AudioSampleRequest {
                        start_frame: start,
                        max_bytes,
                    },
                )?,
            });
        }
        let pending = self.sample_read.as_mut().unwrap();
        ensure!(
            pending.call == *call && pending.channel == channel.resource,
            "native sample query changed"
        );
        let AudioSampleState::Ready(receipt) = pending.job.poll()? else {
            return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
        };
        ensure!(
            receipt.client == owner
                && receipt.resource == channel.resource
                && receipt.start_frame == start,
            "native foreign source sample receipt"
        );
        let result = if name == b"Music_GetFormat" {
            for (arg, value) in args[1..].iter().zip([
                receipt.format.sample_rate as i32,
                i32::from(receipt.format.bits_per_sample),
                i32::from(receipt.format.channels),
            ]) {
                let Argument::Reference { page, index, .. } = arg else {
                    unreachable!()
                };
                m.heap.set(*page, *index, value)?;
            }
            1
        } else {
            let Argument::HeapReference { slot, .. } = args[1] else {
                unreachable!()
            };
            ensure!(
                receipt.bytes.len() <= max_bytes as usize,
                "native source sample extent"
            );
            let target = &mut m.heap.page_mut(slot)?.values;
            for (word, bytes) in target.iter_mut().zip(receipt.bytes.chunks(4)) {
                let mut stored = word.to_le_bytes();
                stored[..bytes.len()].copy_from_slice(bytes);
                *word = i32::from_le_bytes(stored);
            }
            receipt.frames as i32
        };
        m.complete_hll(call, HllReturn::Scalar(result))?;
        self.sample_read = None;
        Ok(Some(Dispatch::Completed))
    }
}
#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;
    use crate::native_audio::tests::{pending, setup};
    use crate::vm::heap::{Object, Page, PageKind};
    fn page(m: &mut Machine, kind: i32, values: Vec<i32>) -> i32 {
        m.heap
            .alloc(Object::Page(Some(Page {
                kind: PageKind::Array {
                    kind,
                    structure: -1,
                    rank: 1,
                },
                values,
            })))
            .unwrap()
    }
    #[test]
    fn native_fft_and_pcm_analysis_write_real_arrays_including_aliases() {
        let (_dir, mut runtime, bridge, mut audio) = setup();
        let (mut m, call) = pending("FFT_rdft", &[1]);
        let output = page(
            &mut m,
            15,
            [0f32, 1., 0., -1.].map(|n| n.to_bits() as i32).to_vec(),
        );
        m.stack = vec![output];
        let f = &mut Arc::make_mut(&mut m.program).libraries[0].functions[0];
        f.return_kind = 0;
        f.arguments = vec![(vec![], 23)];
        assert_eq!(
            audio
                .dispatch(ClientId(1), &mut m, &call, &bridge, &mut runtime.host)
                .unwrap(),
            Some(Dispatch::Completed)
        );
        let values = &m.heap.page(output).unwrap().values;
        for (a, b) in values
            .iter()
            .map(|n| f32::from_bits(*n as u32))
            .zip([0., 0., 0., 2.])
        {
            assert!((a - b).abs() < 1e-5);
        }
        assert!(m.stack.is_empty());
        let (mut m, call) = pending("Music_AnalyzeSampleData", &[1, 1, 2, 2, 16]);
        let output = page(&mut m, 15, vec![99; 4]);
        let input = page(&mut m, 14, vec![i32::from_le_bytes([0, 128, 255, 127])]);
        m.stack = vec![output, output, input, 2, 16];
        let f = &mut Arc::make_mut(&mut m.program).libraries[0].functions[0];
        f.arguments = [23, 23, 22, 10, 10].map(|kind| (vec![], kind)).to_vec();
        assert_eq!(
            audio
                .dispatch(ClientId(1), &mut m, &call, &bridge, &mut runtime.host)
                .unwrap(),
            Some(Dispatch::Completed)
        );
        assert_eq!(m.stack, [1]);
        assert_eq!(
            f32::from_bits(m.heap.page(output).unwrap().values[0] as u32),
            32767. / 32768.
        );
        assert_eq!(m.heap.page(output).unwrap().values[1..], [99; 3]);
    }
    #[test]
    fn native_sample_query_uses_prepared_source_layout_and_keeps_pc_until_worker_ready() {
        let (_dir, mut runtime, bridge, mut audio) = setup();
        // Reuse a genuine shared prepared WAV resource from the existing source
        // fixture; only the native channel namespace changes for this query test.
        let (mut prepare, pcall) = pending("Sound_Prepare", &[3, 1]);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            match audio
                .dispatch(
                    ClientId(1),
                    &mut prepare,
                    &pcall,
                    &bridge,
                    &mut runtime.host,
                )
                .unwrap()
            {
                Some(Dispatch::Completed) => break,
                Some(Dispatch::WaitUntil(_)) => {
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::yield_now()
                }
                _ => panic!("prepare"),
            }
        }
        let channel = audio
            .channels
            .remove(&Key {
                kind: Kind::Sound,
                number: 3,
            })
            .unwrap();
        audio.channels.insert(
            Key {
                kind: Kind::Music,
                number: 3,
            },
            channel,
        );
        let (mut m, call) = pending("Music_GetSampleData", &[3, 1, 0]);
        let output = page(&mut m, 14, vec![0x55555555; 2]);
        m.stack = vec![3, output, 0];
        Arc::make_mut(&mut m.program).libraries[0].functions[0].arguments =
            [10, 22, 10].map(|kind| (vec![], kind)).to_vec();
        loop {
            match audio
                .dispatch(ClientId(1), &mut m, &call, &bridge, &mut runtime.host)
                .unwrap()
            {
                Some(Dispatch::Completed) => break,
                Some(Dispatch::WaitUntil(_)) => {
                    assert_eq!(m.pc, call.pc);
                    assert_eq!(m.stack, [3, output, 0]);
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::yield_now()
                }
                _ => panic!("sample"),
            }
        }
        assert_eq!(m.stack, [2]);
        assert_eq!(m.heap.page(output).unwrap().values, [0x40002000; 2]);
        let (mut m, format_call) = pending("Music_GetFormat", &[3, 1, 0, 1, 1, 1, 2]);
        let output = m
            .heap
            .alloc(Object::Page(Some(Page {
                kind: PageKind::Global,
                values: vec![77; 3],
            })))
            .unwrap();
        let before = vec![3, output, 0, output, 1, output, 2];
        m.stack = before.clone();
        Arc::make_mut(&mut m.program).libraries[0].functions[0].arguments =
            [10, 18, 18, 18].map(|kind| (vec![], kind)).to_vec();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            match audio
                .dispatch(
                    ClientId(1),
                    &mut m,
                    &format_call,
                    &bridge,
                    &mut runtime.host,
                )
                .unwrap()
            {
                Some(Dispatch::Completed) => break,
                Some(Dispatch::WaitUntil(_)) => {
                    assert_eq!(m.pc, format_call.pc);
                    assert_eq!(m.stack, before);
                    assert_eq!(m.heap.page(output).unwrap().values, [77; 3]);
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::yield_now();
                }
                _ => panic!("format"),
            }
        }
        assert_eq!(m.stack, [1]);
        // Existing WAV source fixture encodes48000 Hz,16-bit stereo.
        assert_eq!(m.heap.page(output).unwrap().values, [48000, 16, 2]);
        audio.close(&mut runtime.host).unwrap();
    }
}
