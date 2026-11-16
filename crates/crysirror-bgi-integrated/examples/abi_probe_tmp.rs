fn main() {
    let game = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let user = std::env::temp_dir().join("bgi-abi-probe");
    std::fs::create_dir_all(&user).unwrap();
    let host = crysirror_bgi_integrated::HostRuntime::create(crysirror_bgi_integrated::HostConfig {
        resource_root: game, user_root: user, font_path: Some("/System/Library/Fonts/Supplemental/Arial Unicode.ttf".into()),
        font_data: None, font_face_index: 0, gbk: false, backend: 0, strict: true, trace: false, silent: true,
    }).unwrap();
    println!("{}", &host.diagnostics()[..40]);
}
