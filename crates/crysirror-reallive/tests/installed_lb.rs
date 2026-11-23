// SPDX-License-Identifier: MPL-2.0
//! Opt-in checks against owner-provided data; no game assets are fixtures.
use crysirror_core::service::{InputFrame, buttons};
use crysirror_reallive::vm::{Archive, Gameexe, Nls, bytecode::Element, expr::Expr};
use crysirror_runtime::{AdapterRegistry, Project, Runtime};

struct G00;
impl crysirror_image::ImageDecoder for G00 {
    fn decode(&self, bytes: &[u8]) -> anyhow::Result<crysirror_image::ImageSurface> {
        let atlas = crysirror_image::g00::decode_g00(bytes)?;
        Ok(crysirror_image::ImageSurface {
            width: atlas.width as u32,
            height: atlas.height as u32,
            rgba: atlas.rgba,
        })
    }
}

fn root() -> std::path::PathBuf {
    std::env::var_os("REALLIVE_TEST_GAME")
        .expect("REALLIVE_TEST_GAME required")
        .into()
}
#[test]
#[ignore = "requires owner-provided Little Busters installation"]
fn locate_baseball_scripts() {
    let root = root();
    let exe = Gameexe::from_bytes(&std::fs::read(root.join("Gameexe.ini")).unwrap(), Nls::Sjis);
    let archive = Archive::from_bytes(
        std::fs::read(root.join("Seen.txt")).unwrap(),
        exe.str("REGNAME").unwrap_or(""),
        Nls::Sjis,
    )
    .unwrap();
    let mut count = 0;
    for n in archive.numbers() {
        let scene = archive.scenario(n).unwrap();
        let calls: Vec<_> = scene
            .script
            .elements
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let Element::Command(c) = e else {
                    return None;
                };
                (c.op.modtype == 2
                    && c.op.module == 1
                    && c.op.opcode == 12
                    && c.params.first().is_some_and(|p| p.value == Expr::Int(0)))
                .then_some((i, c.params.get(1).map(|p| &p.value)))
            })
            .collect();
        if !calls.is_empty() {
            count += calls.len();
            println!(
                "PT00 scene {n}: {} calls; first {:?}",
                calls.len(),
                &calls[..calls.len().min(8)]
            );
        }
    }
    assert!(count > 0, "installation has no PT00 call sites");
}

#[test]
#[ignore = "requires owner-provided Little Busters installation and font"]
fn original_baseball_script_runs_through_shared_services() {
    let root = root();
    let scene: i32 = std::env::var("REALLIVE_TEST_SCENE")
        .expect("REALLIVE_TEST_SCENE required")
        .parse()
        .unwrap();
    let font =
        std::fs::read(std::env::var_os("REALLIVE_TEST_FONT").expect("REALLIVE_TEST_FONT required"))
            .unwrap();
    let project: Project = serde_json::from_value(serde_json::json!({"schema":2,"name":"installed LB verification","content_revision":"1","canvas":[800,600],"assets":"game","clients":[{"name":"game","adapter":"reallive","game":"owner-installed","config":{"start_scene":scene}}]})).unwrap();
    let state = tempfile::tempdir().unwrap();
    let mut registry = AdapterRegistry::default();
    crysirror_reallive::register(&mut registry).unwrap();
    let mut runtime = Runtime::with_asset_root(project, &root, state.path(), registry).unwrap();
    runtime
        .assets
        .decoders
        .register("g00", std::sync::Arc::new(G00))
        .unwrap();
    runtime.host.text.add_font("default", font.clone()).unwrap();
    runtime.assets.text.add_font("default", font).unwrap();
    let ticks: u32 = std::env::var("REALLIVE_TEST_TICKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(6000);
    for tick in 0..ticks {
        let mut input = InputFrame {
            pointer_x: 400,
            pointer_y: 300,
            ..Default::default()
        };
        // Fast-forward explanations and exercise batting presses at runtime.
        if tick % 120 < 90 {
            input.buttons_down = buttons::CTRL;
        }
        if tick % 30 == 0 {
            input.buttons_pressed = buttons::MOUSE_LEFT;
            input.buttons_down |= buttons::MOUSE_LEFT;
        }
        runtime.tick(16, input).unwrap_or_else(|e| {
            panic!(
                "tick {tick}: {e:#}; native {}",
                runtime.inspect()["sessions"][0]["native"]
            )
        });
        if tick % 1000 == 0 {
            let n = &runtime.inspect()["sessions"][0]["native"];
            println!(
                "tick {tick}: scene {} line {} wait {}",
                n["scene"], n["line"], n["wait"]
            );
        }
        // Owner-polled immutable resource jobs must get wall time too.
        if tick % 50 == 0 {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    let native = &runtime.inspect()["sessions"][0]["native"];
    println!("final {native}");
    assert!(native["error"].is_null());
    assert!(
        native["opcodes"]["op<2:001:00012, 5>"]
            .as_u64()
            .unwrap_or(0)
            > 0
            || native["opcodes"]
                .as_object()
                .unwrap()
                .keys()
                .any(|k| k.starts_with("op<2:001:00012,")),
        "original script did not call PT00"
    );
}
