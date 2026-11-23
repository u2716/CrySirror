// SPDX-License-Identifier: MPL-2.0
import SwiftUI
import Combine
import UniformTypeIdentifiers
import CryptoKit
import QuartzCore

func nativeJSON(_ pointer: UnsafeMutablePointer<CChar>?) -> [String: Any] {
    guard let pointer else { return ["error": "Native host returned no response"] }
    defer { crysirror_string_free(pointer) }
    let data = Data(String(cString: pointer).utf8)
    return (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] ?? ["error": "Invalid native response"]
}
func jsonText(_ value: [String: Any]) -> String {
    String(data: try! JSONSerialization.data(withJSONObject: value), encoding: .utf8)!
}
struct ViewBinding { var session: UInt64 = 0; var generation: UInt64 = 0 }
final class Player: ObservableObject {
    @Published var source: URL?
    @Published var engine = ""
    @Published var encodings: [String] = []
    @Published var nls = "auto"
    @Published var message = "Choose a game folder."
    @Published var probing = false
    @Published var playing = false
    var session: UInt64 = 0
    var surfaceGeneration: UInt64 = 0
    var scoped = false
    var paused = true
    func select(_ url: URL) {
        stop()
        if scoped { source?.stopAccessingSecurityScopedResource() }
        source = url; scoped = url.startAccessingSecurityScopedResource()
        probing = true; message = "Detecting engine…"; nls = "auto"
        DispatchQueue.global(qos: .userInitiated).async {
            let result = url.path.withCString { nativeJSON(crysirror_probe($0)) }
            DispatchQueue.main.async {
                self.probing = false
                if let error = result["error"] as? String { self.message = error; self.engine = ""; return }
                self.engine = result["engine"] as? String ?? ""
                self.encodings = result["nls_options"] as? [String] ?? []
                self.message = result["title"] as? String ?? url.lastPathComponent
            }
        }
    }
    func bind(layer: CAMetalLayer, size: CGSize, previous: ViewBinding) -> ViewBinding {
        guard playing, let source, size.width > 0, size.height > 0 else { return previous }
        if previous.generation != 0 {
            if previous.session == session && previous.generation == surfaceGeneration {
                command(["op":"resize", "width":Int(size.width), "height":Int(size.height)])
            }
            return previous
        }
        let pointer = Unmanaged.passUnretained(layer).toOpaque()
        let result: [String: Any]
        if session == 0 {
            let identity = SHA256.hash(data: Data(source.path.utf8)).map { String(format: "%02x", $0) }.joined()
            let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            let save = support.appendingPathComponent("CrySirror/Saves/\(identity)")
            let request = jsonText(["game": source.path, "state_dir": save.path, "nls": nls])
            result = request.withCString { nativeJSON(crysirror_start($0, pointer, UInt32(size.width), UInt32(size.height))) }
            session = (result["session"] as? NSNumber)?.uint64Value ?? 0
            command(["op":"pause", "paused": paused])
        } else {
            result = nativeJSON(crysirror_attach(session, pointer, UInt32(size.width), UInt32(size.height)))
        }
        if let error = result["error"] as? String { message = error; return ViewBinding() }
        message = "Loading…"
        surfaceGeneration = (result["surface_generation"] as? NSNumber)?.uint64Value ?? 0
        return ViewBinding(session: session, generation: surfaceGeneration)
    }
    func detach(_ binding: ViewBinding) {
        guard binding.session != 0, binding.session == session, binding.generation != 0 else {return}
        let result = nativeJSON(crysirror_detach(binding.session, binding.generation))
        if let error = result["error"] as? String {message = error}
    }
    func input(_ value: [String: Any], binding: ViewBinding) {
        guard binding.session == session, binding.generation != 0, binding.generation == surfaceGeneration else {return}
        command(value)
    }
    func setPaused(_ value: Bool) {
        paused = value
        command(["op":"pause", "paused": value])
    }
    @discardableResult func command(_ value: [String: Any]) -> [String: Any] {
        guard session != 0 else { return [:] }
        return jsonText(value).withCString { nativeJSON(crysirror_command(session, $0)) }
    }
    func poll() {
        let result = command(["op": "status"])
        switch result["phase"] as? String {
        case "failed": message = result["message"] as? String ?? "Session failed"
        case "running": message = ""
        case "waiting_surface": message = result["message"] as? String ?? "Waiting for display…"
        case "paused": message = "Paused"
        case "stopped": stop()
        default: break
        }
    }
    func stop() {
        let old = session; session = 0; surfaceGeneration = 0
        if old != 0 { crysirror_destroy(old) }
        playing = false
    }
    deinit { if session != 0 { crysirror_destroy(session) }; if scoped { source?.stopAccessingSecurityScopedResource() } }
}
@main struct CrySirrorApp: App {
    @StateObject private var player = Player()
    @State private var importer = false
    @Environment(\.scenePhase) private var phase
    let clock = Timer.publish(every: 0.25, on: .main, in: .common).autoconnect()
    var body: some Scene {
        WindowGroup {
            VStack(spacing: 12) {
                if player.playing {
                    NativePlayer(player: player)
                        .overlay(alignment: .topLeading) {
                            if !player.message.isEmpty { Text(player.message).padding().background(.black.opacity(0.8)).foregroundColor(.white) }
                        }
                    HStack {
                        Button("Menu") {
                            player.command(["op":"input", "down": 1 << 17]); player.command(["op":"input", "down": 0])
                        }
                        Spacer()
                        Button("Close game") { player.command(["op":"close"]) }
                        Button("Library") { player.stop() }
                    }.padding(.horizontal)
                } else {
                    Image("AppMark").resizable().scaledToFit().frame(width: 100, height: 100)
                    Text("CrySirror").font(.largeTitle)
                    Text(player.message).textSelection(.enabled).frame(maxWidth: 560)
                    if !player.engine.isEmpty { Text(player.engine).foregroundStyle(.secondary) }
                    if !player.encodings.isEmpty {
                        Picker("Text encoding", selection: $player.nls) {
                            Text("Automatic").tag("auto")
                            ForEach(player.encodings, id: \.self) { Text($0.uppercased()).tag($0) }
                        }.frame(maxWidth: 320)
                    }
                    HStack {
                        Button("Choose game folder…") { importer = true }.disabled(player.probing)
                        Button("Play") { player.playing = true }.disabled(player.probing || player.engine.isEmpty)
                    }
                }
            }
            .padding(player.playing ? 0 : 24)
            .frame(minWidth: 320, minHeight: 240)
            .fileImporter(isPresented: $importer, allowedContentTypes: [.folder]) { result in
                if case .success(let url) = result { player.select(url) }
            }
            .onReceive(clock) { _ in if player.session != 0 { player.poll() } }
            .onAppear { player.setPaused(phase != .active) }
            .onChange(of: phase) { value in player.setPaused(value != .active) }
        }
    }
}
