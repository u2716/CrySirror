// Copyright (c) 2026 CrySirror contributors.
// Distributed under the GNU Affero General Public License, version 3 or
// (at your option) any later version, https://www.gnu.org/licenses/agpl-3.0.html.
//! lzfs / yuzuex wrapper media, getLangName and the option-only plug-ins.
//! Self-authored fixtures only.
use crysirror_core::service::InputFrame;
use crysirror_runtime::{AdapterRegistry, Runtime};
use serde_json::json;
use std::io::Write;

fn run(files: &[(&str, Vec<u8>)], config: serde_json::Value, marker: &str) {
    let temp = tempfile::tempdir().unwrap();
    let assets = temp.path().join("game");
    for (name, bytes) in files {
        let path = assets.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let mut config = config;
    config["canvas"] = json!([64, 64]);
    let project = serde_json::from_value(json!({"schema":2,"name":"wrappers",
        "content_revision":"1","assets":"game","canvas":[64,64],
        "clients":[{"name":"game","game":"authored","adapter":"krkr","config":config}]}))
    .unwrap();
    let mut registry = AdapterRegistry::default();
    crysirror_krkr::register(&mut registry).unwrap();
    let mut rt = Runtime::with_asset_root(project, &assets, &temp.path().join("state"), registry).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        rt.tick(1, InputFrame::default()).unwrap();
        let info = rt.inspect();
        if info["sessions"][0]["native"]["log_tail"].to_string().contains(marker) {
            return;
        }
        assert!(std::time::Instant::now() < deadline, "{info}");
        std::thread::yield_now();
    }
}

#[test]
fn lzfs_serves_frames_decoded_and_other_files_raw() {
    let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
    encoder.write_all("packed text 圧縮".as_bytes()).unwrap();
    let packed = encoder.finish().unwrap();
    run(
        &[
            ("data/packed.txt", packed),
            ("data/plain.txt", "plain text".as_bytes().to_vec()),
            (
                "startup.tjs",
                r#"
                var missing = false;
                try { Storages.getFullPath("lzfs://./x"); } catch(e) { missing = true; }
                if (!missing) throw "lzfs before link";
                Plugins.link("lzfs.dll");
                Storages.addAutoPath("data/");
                if (!Storages.isExistentStorage("lzfs://./packed.txt")) throw "lzfs miss";
                if ([].load("lzfs://./packed.txt")[0] !== "packed text 圧縮") throw "not decoded";
                if ([].load("lzfs://./plain.txt")[0] !== "plain text") throw "not raw";
                if (Storages.isExistentStorage("lzfs://./absent.txt")) throw "invented file";
                Debug.message("lzfs passed");
            "#
                .as_bytes()
                .to_vec(),
            ),
        ],
        json!({}),
        "lzfs passed",
    );
}

#[test]
fn proxy_media_maps_exact_lowercased_keys_through_auto_paths() {
    run(
        &[
            ("plugin/target.txt", b"target".to_vec()),
            (
                "startup.tjs",
                br#"
                Plugins.link("yuzuex.dll");
                if (typeof global.ProxyStorageMap != "Object") throw "map absent";
                ProxyStorageMap["./alias.txt"] = System.exePath + "plugin/target.txt";
                ProxyStorageMap["./thumb/a"] = <% 00 01 %>;
                Storages.addAutoPath("proxy://./");
                if ([].load("proxy://./ALIAS.txt")[0] !== "target") throw "proxy read";
                if ([].load("alias.txt")[0] !== "target") throw "auto path";
                if (Storages.isExistentStorage("proxy://./thumb/a")) throw "octet mapped";
                if (Storages.isExistentStorage("proxy://./absent")) throw "absent mapped";
                Debug.message("proxy passed");
            "#
                .to_vec(),
            ),
        ],
        json!({}),
        "proxy passed",
    );
}

#[test]
fn lang_name_follows_configured_locale_and_option_plugins_link() {
    run(
        &[(
            "startup.tjs",
            br#"
            Plugins.link("getLangName.dll");
            if (System.getCurrentUILangName() !== "ja") throw "ui " + System.getCurrentUILangName();
            if (System.getCurrentLocaleName() !== "ja-JP") throw "locale";
            Plugins.link("kagexopt.dll");
            Plugins.link("wuopus.dll");
            if (Plugins.getStatus("kagexopt.dll").kind != "engine_facade") throw "kagexopt";
            Debug.message("lang passed");
        "#
            .to_vec(),
        )],
        json!({"locale": "ja_JP.UTF-8"}),
        "lang passed",
    );
}

fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let mut out = Vec::new();
    out.extend(b"RIFF");
    out.extend((36 + data.len() as u32).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(rate.to_le_bytes());
    out.extend((rate * 2).to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    out
}

#[test]
fn get_sample_reads_the_playing_window() {
    // A constant positive level, then silence: the reference average of the
    // non-negative samples is the level itself.
    let samples: Vec<i16> = (0..44100 * 4).map(|i| if i % 2 == 0 { 1000 } else { -1000 }).collect();
    run(
        &[
            ("voice.wav", wav(&samples, 44100)),
            (
                "startup.tjs",
                br#"
                Plugins.link("getSample.dll");
                var w = new WaveSoundBuffer(null);
                if (w.getSample() !== 0) throw "stopped buffer has samples";
                if (w.sampleCount !== 100 || w.sampleAhead !== 0) throw "defaults";
                WaveSoundBuffer.setDefaultCounts(50);
                var w2 = new WaveSoundBuffer(null);
                if (w2.sampleCount !== 50) throw "static default";
                w.open("voice.wav");
                w.play();
                var t = new Timer(function() {
                    var level = w.getSample(64);
                    var peak = w.sampleValue;
                    if (level == 0) return;
                    if (level !== 1000) throw "level " + level;
                    if (Math.abs(peak - (1000.0/32768)*(1000.0/32768)) > 1e-9) throw "peak " + peak;
                    t.enabled = false;
                    Debug.message("getSample passed");
                }, "");
                t.interval = 1;
                t.enabled = true;
            "#
                .to_vec(),
            ),
        ],
        json!({}),
        "getSample passed",
    );
}

#[test]
fn launcher_draw_device_and_compat_plugins_report_their_profiles() {
    run(
        &[(
            "startup.tjs",
            br#"
            var w = new Window();
            Plugins.link("k2compat.dll");
            var hits = 0;
            w.onMouseDown = function(x, y, b, s) { if (x == 3 && y == 4) hits++; };
            var tm = new Window.TouchMouse(w);
            if (tm.enabled !== 0) throw "touch default";
            tm.enabled = true;
            tm.onTouchMouseDown(3, 4, 0, 0);
            if (hits != 1) throw "touch forward";
            var owner = new System.ModelessOwnerWindow();
            owner.show(true);
            owner.stayOnTop = true;
            if (owner.stayOnTop !== 1) throw "owner";
            Plugins.link("dmmcloud.dll");
            if ((new DMMCloud()).open_store("x") !== 0) throw "dmm";
            Plugins.link("krkrsteam.dll");
            if (Steam.requestInitialize() !== 0 || Steam.initialized !== 0) throw "steam";
            if (Steam.getLanguage() !== "japanese") throw "steam language";
            if (Steam.getAchievement("A").achieved !== 0) throw "achievement record";
            var refused = false; try { new Steam(); } catch(e) { refused = true; }
            if (!refused) throw "steam instance";
            Plugins.link("DrawDeviceD2D.dll");
            Plugins.link("SteamDrawDevice.dll");
            w.drawDevice = new DrawDeviceD2D();
            w.drawDevice.recreate();
            w.drawDevice = new DualDrawDevice_Z();
            Plugins.link("win32ole.dll");
            var voice = new WIN32OLE("SAPI.SpVoice");
            var voices;
            try { voices = voice.invoke("GetVoices"); } catch(e) { voices = "failed"; }
            if (voices !== "failed") throw "ole transport invented";
            voice.addEvent("Word", w);
            var x = new ActiveX("Shell.Explorer", w, 1, 2, 30, 40);
            if (x.width !== 30 || x.visible !== 0) throw "activex geometry";
            Debug.message("compat passed");
        "#
            .to_vec(),
        )],
        json!({"locale": "ja-JP"}),
        "compat passed",
    );
}

#[test]
fn http_request_reaches_only_authorized_origins() {
    use std::io::{Read as _, Write as _};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let mut stream = stream.unwrap();
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let body = "こんにちは";
            let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nX-Test: yes\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    let temp = tempfile::tempdir().unwrap();
    let assets = temp.path().join("game");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(
        assets.join("startup.tjs"),
        format!(
            r#"
            var w = new Window();
            Plugins.link("httprequest.dll");
            global.states = [];
            global.ok = new HttpRequest(w);
            var ok = global.ok;
            ok.onReadyStateChange = function(s) {{
                var ok = global.ok;
                global.states.add(s);
                if (s != HttpRequest.LOADED) return;
                if (ok.status !== 200 || ok.statusText !== "OK") throw "status " + ok.status + " " + ok.statusText;
                if (ok.response !== "こんにちは") throw "body " + ok.response;
                if (ok.getResponseHeader("x-test") !== "yes") throw "header";
                if (ok.contentTypeEncoding !== "utf-8") throw "charset";
                global.denied = new HttpRequest(w);
                global.denied.onReadyStateChange = function(s) {{
                    if (s != HttpRequest.LOADED) return;
                    if (global.denied.status !== 0) throw "unauthorized origin reached";
                    Debug.message("http passed " + global.states.join(","));
                }};
                global.denied.open("GET", "http://127.0.0.2:{port}/");
                global.denied.send();
            }};
            ok.open("GET", "http://127.0.0.1:{port}/hello");
            ok.send();
        "#
        ),
    )
    .unwrap();
    let project = serde_json::from_value(json!({"schema":2,"name":"http",
        "content_revision":"1","assets":"game","canvas":[64,64],
        "clients":[{"name":"game","game":"authored","adapter":"krkr","config":{"canvas":[64,64]}}]}))
    .unwrap();
    let mut registry = AdapterRegistry::default();
    crysirror_krkr::register(&mut registry).unwrap();
    let mut rt = Runtime::with_asset_root(project, &assets, &temp.path().join("state"), registry).unwrap();
    rt.host.grant_http_origin(crysirror_core::state::ClientId(1), &format!("http://127.0.0.1:{port}")).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        rt.tick(1, InputFrame::default()).unwrap();
        let log = rt.inspect()["sessions"][0]["native"]["log_tail"].to_string();
        if log.contains("http passed 1,2,3,4") {
            return;
        }
        assert!(std::time::Instant::now() < deadline, "{}", rt.inspect());
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
