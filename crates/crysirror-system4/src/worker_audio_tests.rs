// Original AIN HLL -> shared Audio acceptance; GPL-2.0-or-later.
use super::*;
use crate::ain::HllFunction;
use crysirror_core::service::AudioRequest;
use std::time::{Duration, Instant};
fn actor() -> (tempfile::TempDir, crysirror_runtime::Runtime, NativeAdapter) {
    let (t, runtime, bridge, audio) = crate::native_audio::tests::setup();
    let (mut machine, call) = crate::native_audio::tests::pending("Sound_Prepare", &[7, 1]);
    let program = Arc::make_mut(&mut machine.program);
    program.code.truncate(program.code.len() - 2);
    program.libraries[0].functions.push(HllFunction {
        name: b"Sound_Play".to_vec(),
        return_kind: 10,
        arguments: vec![(vec![], 10)],
    });
    let mut op = |code: u16, args: &[i32]| {
        program.code.extend(code.to_le_bytes());
        for arg in args {
            program.code.extend(arg.to_le_bytes());
        }
    };
    op(1, &[]);
    op(0, &[7]);
    op(0x5a, &[0, 1]);
    op(1, &[]);
    for _ in 0..32 {
        op(0, &[0]);
        op(1, &[]);
    }
    op(0, &[0]);
    op(0x2f, &[]);
    let mut vm = NativeAdapter::new(
        ClientId(1),
        Config {
            program: "game.ain".into(),
            ini: None,
            volumes: vec![],
            canvas: Some([1, 1]),
            font_aliases: ["g".into(), "m".into()],
            reviewed_output_log_dll: None,
        },
    )
    .unwrap();
    vm.started = true;
    vm.boot_ready = true;
    vm.bridge = bridge;
    vm.native_audio = audio;
    vm.machine = Some(machine);
    vm.stop = Some(Step::External(call));
    (t, runtime, vm)
}
#[test]
fn worker_dispatches_original_hll_through_shared_pcm_and_retires_it_on_both_halt_and_close() {
    for close in [false, true] {
        let (_t, mut runtime, mut vm) = actor();
        let end = Instant::now() + Duration::from_secs(10);
        let mut heard = false;
        let mut halted = false;
        while Instant::now() < end {
            let result = vm
                .step(&mut runtime.host, InputFrame::default(), 1)
                .unwrap();
            if !heard
                && runtime
                    .host
                    .audio_info(ClientId(1), 0x5200_0007)
                    .unwrap()
                    .is_some_and(|info| info.transport_active)
            {
                runtime.host.channel_audio.render(1).unwrap();
                let output = runtime.host.channel_audio.take_output();
                assert_eq!(output.len(), 48);
                for frame in output {
                    assert_eq!(frame, [0.25, 0.5]);
                }
                heard = true;
                if close {
                    assert!(vm.request_close());
                }
            }
            if result == VmYield::Halted {
                halted = true;
                break;
            }
            std::thread::yield_now();
        }
        assert!(heard && halted);
        assert!(!vm.native_audio.has_state());
        assert!(runtime
            .host
            .audio_info(ClientId(1), 0x5200_0007)
            .unwrap()
            .is_none());
        assert!(!vm.request_close());
        runtime
            .host
            .submit_audio(
                ClientId(1),
                AudioRequest::Stop {
                    channel: 0x5200_0007,
                    fade_ms: 0.,
                },
            )
            .unwrap();
    }
}

#[test]
fn worker_native_error_retires_canonical_audio_without_acknowledging_the_original_call() {
    let (_t, mut runtime, mut vm) = actor();
    Arc::make_mut(&mut vm.machine.as_mut().unwrap().program).libraries[0].functions[1].name =
        b"Sound_AuthoredMissing".to_vec();
    let end = Instant::now() + Duration::from_secs(10);
    let error = loop {
        match vm.step(&mut runtime.host, InputFrame::default(), 1) {
            Err(error) => break error,
            Ok(_) => assert!(Instant::now() < end),
        }
        std::thread::yield_now();
    };
    assert!(
        format!("{error}").contains("Sound_AuthoredMissing")
            || format!("{error}").contains("AuthoredMissing")
    );
    let Some(Step::External(call)) = vm.stop.as_ref() else {
        panic!("native call lost");
    };
    let machine = vm.machine.as_ref().unwrap();
    assert_eq!(machine.pc, call.pc);
    assert_eq!(machine.stack, [7]);
    assert!(!vm.native_audio.has_state());
    assert!(runtime
        .host
        .audio_info(ClientId(1), 0x5200_0007)
        .unwrap()
        .is_none());
}
