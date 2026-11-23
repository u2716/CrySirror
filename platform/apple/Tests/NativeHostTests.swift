// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
import XCTest
import QuartzCore
#if os(macOS)
import AppKit
#else
import UIKit
#endif

@MainActor
final class NativeHostTests: XCTestCase {
    private func decode(_ pointer: UnsafeMutablePointer<CChar>?) throws -> [String: Any] {
        let pointer = try XCTUnwrap(pointer)
        defer { crysirror_string_free(pointer) }
        let value = try JSONSerialization.jsonObject(with: Data(String(cString: pointer).utf8))
        return try XCTUnwrap(value as? [String: Any])
    }
    private func json(_ value: [String: Any]) throws -> String {
        String(decoding: try JSONSerialization.data(withJSONObject: value), as: UTF8.self)
    }
    private func command(_ id: UInt64, _ value: [String: Any]) throws -> [String: Any] {
        let response = try json(value).withCString { try decode(crysirror_command(id, $0)) }
        XCTAssertNil(response["error"], String(describing: response))
        return response
    }
    private func status(_ id: UInt64) throws -> [String: Any] {
        try command(id, ["op": "status"])
    }
    @discardableResult
    private func wait(_ id: UInt64, until predicate: ([String: Any]) -> Bool) throws -> [String: Any] {
        let deadline = Date().addingTimeInterval(30)
        while Date() < deadline {
            let state = try status(id)
            if state["phase"] as? String == "failed" {
                XCTFail(String(describing: state))
                throw NSError(domain: "CrySirrorHost", code: 1)
            }
            if predicate(state) { return state }
            RunLoop.current.run(until: Date().addingTimeInterval(0.01))
        }
        XCTFail("Host deadline: \(try status(id))")
        throw NSError(domain: "CrySirrorHost", code: 2)
    }
    private func fixture() throws -> URL {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root.appendingPathComponent("assets"), withIntermediateDirectories: true)
        // Original generated two-colour BMP; no owner game data or fonts.
        var bmp = Data()
        func word(_ value: UInt16) { bmp.append(contentsOf: [UInt8(value & 255), UInt8(value >> 8)]) }
        func dword(_ value: UInt32) { word(UInt16(value & 65535)); word(UInt16(value >> 16)) }
        word(0x4d42); dword(70); dword(0); dword(54); dword(40)
        dword(2); dword(2); word(1); word(24); dword(0); dword(16)
        dword(0); dword(0); dword(0); dword(0)
        bmp.append(contentsOf: [0, 190, 80, 0, 100, 230, 0, 0, 0, 100, 230, 0, 190, 80, 0, 0])
        try bmp.write(to: root.appendingPathComponent("assets/card.bmp"))
        // A quiet generated tone opens the real PCM device (all-zero PCM is lazy).
        var wav = Data("RIFF".utf8)
        func wavWord(_ value: UInt16) { wav.append(contentsOf: [UInt8(value & 255), UInt8(value >> 8)]) }
        func wavDword(_ value: UInt32) { wavWord(UInt16(value & 65535)); wavWord(UInt16(value >> 16)) }
        wavDword(9636); wav.append(Data("WAVEfmt ".utf8)); wavDword(16)
        wavWord(1); wavWord(1); wavDword(48000); wavDword(96000); wavWord(2); wavWord(16)
        wav.append(Data("data".utf8)); wavDword(9600)
        for sample in 0..<4800 {
            let value = Int16(64 * sin(Double(sample) * 2 * .pi * 440 / 48000))
            wavWord(UInt16(bitPattern:value))
        }
        try wav.write(to:root.appendingPathComponent("assets/tone.wav"))
        let sprite: [String: Any] = ["op":"sprite", "id":1, "asset":"card.bmp", "rect":[40,40,160,120], "z":1, "tint":[255,255,255,255]]
        let project: [String: Any] = ["schema":2, "name":"Native host lifecycle", "content_revision":"1", "canvas":[320,240], "background":[20,40,70,255], "assets":"assets", "clients":[["name":"fixture", "adapter":"crysirror-script-v1", "game":"fixture", "program":[sprite, ["op":"play", "channel":1, "asset":"tone.wav"], ["op":"wait_input"], ["op":"wait_input"]]]]]
        try JSONSerialization.data(withJSONObject: project).write(to: root.appendingPathComponent("project.json"))
        return root
    }
    func testMetalSurfacePauseAndReplacementRetainsSession() throws {
        let root = try fixture()
        defer { try? FileManager.default.removeItem(at: root) }
        var id: UInt64 = 0
        var surfaces: [NativeTestSurface] = []
        defer {
            if id != 0 { crysirror_destroy(id) }
            surfaces.forEach { $0.close() }
        }
        let first = try NativeTestSurface(); surfaces.append(first)
        let request = try json(["game":root.appendingPathComponent("project.json").path, "state_dir":root.appendingPathComponent("state").path, "muted":false])
        let started = try request.withCString { try decode(crysirror_start($0, Unmanaged.passUnretained(first.layer).toOpaque(), 320, 240)) }
        XCTAssertNil(started["error"], String(describing: started))
        id = try XCTUnwrap(started["session"] as? NSNumber).uint64Value
        let generation = try XCTUnwrap(started["surface_generation"] as? NSNumber).uint64Value
        let initial = try wait(id) { ($0["frames"] as? NSNumber)?.uint64Value ?? 0 >= 30 }
        XCTAssertEqual(initial["attached"] as? Bool, true)
        _ = try command(id, ["op":"pause", "paused":true])
        let paused = try wait(id) { $0["phase"] as? String == "paused" }
        RunLoop.current.run(until: Date().addingTimeInterval(0.12))
        XCTAssertEqual((try status(id))["frames"] as? NSNumber, paused["frames"] as? NSNumber)
        _ = try decode(crysirror_detach(id, generation))
        _ = try wait(id) { $0["attached"] as? Bool == false }
        let second = try NativeTestSurface(); surfaces.append(second)
        let attached = try decode(crysirror_attach(id, Unmanaged.passUnretained(second.layer).toOpaque(), 320, 240))
        XCTAssertNil(attached["error"], String(describing: attached))
        let next = try XCTUnwrap(attached["surface_generation"] as? NSNumber).uint64Value
        XCTAssertGreaterThan(next, generation)
        _ = try decode(crysirror_detach(id, generation)) // Delayed old-view callback.
        _ = try command(id, ["op":"pause", "paused":false])
        let before = (paused["frames"] as? NSNumber)?.uint64Value ?? 0
        let resumed = try wait(id) {
            ($0["surface_generation"] as? NSNumber)?.uint64Value == next &&
            (($0["frames"] as? NSNumber)?.uint64Value ?? 0) > before + 3
        }
        XCTAssertEqual(resumed["attached"] as? Bool, true)
        let destroyed = id; crysirror_destroy(id); id = 0
        let stale = try "{\"op\":\"status\"}".withCString { try decode(crysirror_command(destroyed, $0)) }
        XCTAssertNotNil(stale["error"])
    }
}

@MainActor
private final class NativeTestSurface {
    let layer = CAMetalLayer()
    #if os(macOS)
    let window: NSWindow
    #else
    let window: UIWindow
    #endif
    init() throws {
        let frame = CGRect(x:0, y:0, width:320, height:240)
        #if os(macOS)
        window = NSWindow(contentRect:frame, styleMask:.borderless, backing:.buffered, defer:false)
        window.isReleasedWhenClosed = false
        let view = NSView(frame:frame); view.wantsLayer = true; view.layer = layer
        window.contentView = view
        window.orderBack(nil)
        #else
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.first as? UIWindowScene)
        window = UIWindow(windowScene:scene); window.frame = frame
        let controller = UIViewController(); controller.view.frame = frame
        controller.view.layer.addSublayer(layer)
        window.rootViewController = controller; window.makeKeyAndVisible()
        #endif
        layer.frame = frame; layer.drawableSize = frame.size; layer.contentsScale = 1
    }
    func close() {
        #if os(macOS)
        window.close()
        #else
        window.isHidden = true
        #endif
    }
}
