//! Actual engine-owned resource/UI execution, without window/GPU/audio device.
use anyhow::{Context, Result};
use crysirror_core::service::{buttons, InputFrame};
use crysirror_runtime::{model::Project, runtime::Runtime, AdapterRegistry};
use serde_json::json;
fn main() -> Result<()> {
    let game = std::env::args()
        .nth(1)
        .context("headless_boot GAME_ROOT WRITABLE_DIR")?;
    let writable = std::env::args()
        .nth(2)
        .context("private writable directory required")?;
    let confirm_empty = std::env::args().any(|a| a == "--confirm-empty-profile");
    let profile=std::env::args().position(|argument|argument=="--profile").map(|index|std::env::args().nth(index+1).context("--profile needs dc4ph or dc3wy")).transpose()?.unwrap_or_else(||"dc4ph".into());
    let (canvas,abi,game_id)=match profile.as_str(){"dc4ph"=>([1280,720],"dc4ph-abb6","DC4PHDL"),"dc3wy"=>([1024,576],"dc3wy-a09f","DC3WY"),"dc2pc"=>([800,600],"dc2pc-5769","DC2PC"),_=>anyhow::bail!("unsupported explicit Circus native profile {profile}")};
    let project: Project = serde_json::from_value(
        json!({"schema":2,"name":"Circus native headless verification","content_revision":abi,"canvas":canvas,"assets":"Advdata","clients":[{"name":"native","adapter":"circus","game":game_id,"config":{"script":"start","font":"native-font","abi":abi}}]}),
    )?;
    let mut registry = AdapterRegistry::default();
    crysirror_circus::register(&mut registry)?;
    let mut runtime = Runtime::with_asset_root(
        project,
        std::path::Path::new(&game),
        std::path::Path::new(&writable),
        registry,
    )?;
    let owner = runtime.inspect()["sessions"][0]["client"]
        .as_u64()
        .context("native owner")? as u32;
    if let Some(index)=std::env::args().position(|argument|argument=="--font") {
        let path=std::env::args().nth(index+1).context("--font requires a caller-selected licensed font")?;
        let bytes=std::fs::read(path)?;
        runtime.host.text.add_font("native-font",bytes.clone())?;
        runtime.assets.text.add_font("native-font",bytes)?;
    }
    let timing=std::env::args().any(|argument|argument=="--timing");
    let dialogue_clicks=std::env::args().any(|argument|argument=="--dialogue-clicks");
    let mut profile_input = 0;
    let activate_title=std::env::args().any(|a|a=="--activate-title");
    let mut title_wait=0;
    let mut title_clicked=false;
    let frame_limit=std::env::args().position(|argument|argument=="--frames").and_then(|i|std::env::args().nth(i+1)).map(|v|v.parse::<u32>()).transpose()?.unwrap_or(10000);
    for frame in 0..frame_limit {
        // Runtime's offline mixer advances audio transport without opening a
        // device; native voice/SE completion must share this engine clock.
        let mut input = InputFrame::default();
        if confirm_empty {
            // Authored input stimulus through ordinary shared UI dispatch;
            // it never injects an opcode/VM flag or skips a native command.
            match profile_input {
                0 if runtime.host.engine.ui.get(owner, 0x3800_0003).is_some() => {
                    anyhow::ensure!(
                        runtime
                            .host
                            .engine
                            .ui
                            .get(owner, 0x3800_0001)
                            .is_some_and(|w| w.text.is_empty())
                            && runtime
                                .host
                                .engine
                                .ui
                                .get(owner, 0x3800_0002)
                                .is_some_and(|w| w.text.is_empty()),
                        "fixture requires an empty private user profile"
                    );
                    input.pointer_x = 1100;
                    input.pointer_y = 640;
                    input.buttons_down = buttons::MOUSE_LEFT;
                    input.buttons_pressed = buttons::MOUSE_LEFT;
                    profile_input = 1;
                    eprintln!("native profile: press submit at frame {frame}");
                }
                1 => {
                    input.pointer_x = 1100;
                    input.pointer_y = 640;
                    profile_input = 2;
                }
                2 if runtime.host.engine.ui.dialogs.get(owner).is_some() => {
                    input.buttons_down = buttons::ENTER;
                    input.buttons_pressed = buttons::ENTER;
                    profile_input = 3;
                    eprintln!("native profile: confirm empty values at frame {frame}");
                }
                _ => {}
            }
        }
        if activate_title&&!title_clicked {
            if let Some(widget)=runtime.host.engine.ui.get(owner,0x36ff_8000){
                title_wait+=1;
                if title_wait>=25&&runtime.host.engine.ui.motion_status(crysirror_core::state::ClientId(owner),0x36ff_fffd)?.complete{
                    let mask=widget.hit_mask.as_ref().context("real title first button needs engine coverage")?;
                    let index=mask.coverage.iter().enumerate().find(|(i,v)|**v>=mask.threshold&&i%mask.size[0] as usize>0&&i/mask.size[0] as usize>0).context("title button has no covered pixel")?.0;
                    input.pointer_x=widget.rect[0] as i32+(index%mask.size[0] as usize) as i32;
                    input.pointer_y=widget.rect[1] as i32+(index/mask.size[0] as usize) as i32;
                    input.buttons_down=buttons::MOUSE_LEFT;input.buttons_pressed=buttons::MOUSE_LEFT;title_clicked=true;
                    eprintln!("real title: pointer press at {},{} frame {}",input.pointer_x,input.pointer_y,frame);
                }
            }
        }else if activate_title&&title_clicked&&input.pointer_x==0{
            // Release the same actual covered location on the following tick.
            if let Some(widget)=runtime.host.engine.ui.get(owner,0x36ff_8000){
                let mask=widget.hit_mask.as_ref().unwrap();let index=mask.coverage.iter().enumerate().find(|(i,v)|**v>=mask.threshold&&i%mask.size[0] as usize>0&&i/mask.size[0] as usize>0).unwrap().0;
                input.pointer_x=widget.rect[0] as i32+(index%mask.size[0] as usize) as i32;input.pointer_y=widget.rect[1] as i32+(index/mask.size[0] as usize) as i32;
            }
        }
        if dialogue_clicks&&title_clicked&&frame%120==0&&runtime.host.engine.layers.get(crysirror_core::state::ClientId(owner),0x39000000).is_some(){
            input.buttons_down|=buttons::MOUSE_LEFT;input.buttons_pressed|=buttons::MOUSE_LEFT;
        }
        let tick_started = std::time::Instant::now();
        let result = runtime.tick(16, input);
        let tick_ms = tick_started.elapsed().as_secs_f64() * 1000.0;
        if timing && tick_ms >= 8.0 {
            eprintln!("slow tick: frame {frame} {tick_ms:.2} ms");
        }
        if let Err(e) = result {
            let effects = runtime.vfx.frame();
            eprintln!(
                "completed engine VFX: emitters={}, particles={}, sequence_receipts={}",
                effects.emitters.len(),
                effects
                    .emitters
                    .iter()
                    .map(|e| e.particles.len())
                    .sum::<usize>(),
                serde_json::to_string(&effects.sequence_states)?
            );
            let published=effects.emitters.iter().filter_map(|emitter| {
                let target=emitter.screen.as_ref()?;
                let frame=runtime.host.engine.layers.dynamic_puppet_frame(emitter.client,target.layer)?;
                Some(json!({"owner":emitter.client,"layer":target.layer,"parts":frame.parts.len(),"textures":frame.textures.len()}))
            }).collect::<Vec<_>>();
            eprintln!(
                "engine screen mesh publications: {}",
                serde_json::to_string(&published)?
            );
            println!("{}", serde_json::to_string_pretty(&runtime.inspect())?);
            return Err(e.context(format!("headless frame {frame}")));
        }
        if runtime.vm_sessions_halted() {
            println!("{}", serde_json::to_string_pretty(&runtime.inspect())?);
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    println!("{}", serde_json::to_string_pretty(&runtime.inspect())?);
    anyhow::bail!("native verification frame limit")
}
