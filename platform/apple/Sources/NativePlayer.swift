// SPDX-License-Identifier: MPL-2.0
import SwiftUI
import QuartzCore
#if os(macOS)
import AppKit
struct NativePlayer: NSViewRepresentable {
    let player: Player
    func makeNSView(context: Context) -> MetalView { MetalView(player: player) }
    func updateNSView(_ view: MetalView, context: Context) {}
    static func dismantleNSView(_ view: MetalView, coordinator: ()) { view.retire() }
}
final class MetalView: NSView {
    let player: Player
    var binding = ViewBinding()
    var retired = false
    let metal = CAMetalLayer()
    var down: UInt64 = 0
    var pointer = CGPoint.zero
    init(player: Player) { self.player = player; super.init(frame: .zero); wantsLayer = true; layer = metal; metal.isOpaque = true }
    required init?(coder: NSCoder) { fatalError("init(coder:)") }
    override var acceptsFirstResponder: Bool { true }
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window == nil { releaseSurface() } else { window?.makeFirstResponder(self); resize() }
    }
    func releaseSurface() { player.detach(binding); binding = ViewBinding(); down = 0 }
    func retire() { retired = true; releaseSurface() }
    override func layout() { super.layout(); resize() }
    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        for area in trackingAreas { removeTrackingArea(area) }
        addTrackingArea(NSTrackingArea(rect: bounds, options: [.mouseMoved,.activeInKeyWindow,.inVisibleRect], owner:self))
    }
    override func mouseMoved(with event: NSEvent) { mouse(event,mask:0,pressed:false) }
    func resize() {
        guard window != nil, !retired else {return}
        let scale = window?.backingScaleFactor ?? 1
        metal.contentsScale = scale
        let size = CGSize(width: bounds.width * scale, height: bounds.height * scale)
        guard size.width > 0, size.height > 0 else { return }
        metal.drawableSize = size
        DispatchQueue.main.async { [weak self] in
            guard let self, !self.retired, self.window != nil else {return}
            self.binding = self.player.bind(layer:self.metal, size:size, previous:self.binding)
        }
    }
    func mouse(_ event: NSEvent, mask: UInt64, pressed: Bool) {
        let p = convert(event.locationInWindow, from: nil); let scale = metal.contentsScale
        pointer = CGPoint(x:p.x*scale,y:(bounds.height-p.y)*scale)
        if pressed { down |= mask } else { down &= ~mask }
        player.input(["op":"input", "x":pointer.x,"y":pointer.y,"down":down], binding:binding)
    }
    override func mouseDown(with event: NSEvent) { mouse(event,mask:1<<16,pressed:true) }
    override func mouseUp(with event: NSEvent) { mouse(event,mask:1<<16,pressed:false) }
    override func mouseDragged(with event: NSEvent) { mouse(event,mask:1<<16,pressed:true) }
    override func rightMouseDown(with event: NSEvent) { mouse(event,mask:1<<17,pressed:true) }
    override func rightMouseUp(with event: NSEvent) { mouse(event,mask:1<<17,pressed:false) }
    func key(_ event: NSEvent, pressed: Bool) {
        let mask: UInt64
        switch event.keyCode {
        case 36: mask=1; case 49: mask=2; case 123: mask=1<<2; case 124: mask=1<<3
        case 126: mask=1<<4; case 125: mask=1<<5; case 53: mask=1<<19
        case 48: mask=1<<6; case 51: mask=1<<20
        default: return
        }
        if pressed { down |= mask } else { down &= ~mask }
        player.input(["op":"input", "x":pointer.x,"y":pointer.y,"down":down], binding:binding)
    }
    override func flagsChanged(with event: NSEvent) {
        down &= ~((1<<11)|(1<<12))
        if event.modifierFlags.contains(.shift) {down |= 1<<11}
        if event.modifierFlags.contains(.control) {down |= 1<<12}
        player.input(["op":"input", "x":pointer.x,"y":pointer.y,"down":down], binding:binding)
    }
    override func keyDown(with event: NSEvent) { key(event,pressed:true) }
    override func keyUp(with event: NSEvent) { key(event,pressed:false) }
}
#else
import UIKit
struct NativePlayer: UIViewRepresentable {
    let player: Player
    func makeUIView(context: Context) -> MetalView { MetalView(player:player) }
    func updateUIView(_ view: MetalView, context: Context) {}
    static func dismantleUIView(_ view: MetalView, coordinator: ()) { view.retire() }
}
final class MetalView: UIView {
    override class var layerClass: AnyClass { CAMetalLayer.self }
    let player: Player
    var binding = ViewBinding()
    var retired = false
    func releaseSurface() {player.detach(binding);binding=ViewBinding()}
    func retire() {retired=true;releaseSurface()}
    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil {releaseSurface()} else {setNeedsLayout()}
    }
    init(player: Player) { self.player=player;super.init(frame:.zero);isMultipleTouchEnabled=true;isOpaque=true }
    required init?(coder:NSCoder) { fatalError("init(coder:)") }
    override func layoutSubviews() {
        super.layoutSubviews()
        guard window != nil, !retired else {return}
        let metal=layer as! CAMetalLayer
        let scale=window?.screen.scale ?? contentScaleFactor
        metal.contentsScale=scale
        let size=CGSize(width:bounds.width*scale,height:bounds.height*scale)
        guard size.width>0 && size.height>0 else {return}
        metal.drawableSize=size
        DispatchQueue.main.async { [weak self] in
            guard let self, !self.retired, self.window != nil else {return}
            self.binding=self.player.bind(layer:metal,size:size,previous:self.binding)
        }
    }
    func input(_ touches:Set<UITouch>,down:Bool) {
        guard let point=touches.first?.location(in:self) else {return}
        let scale=(layer as! CAMetalLayer).contentsScale
        player.input(["op":"input","x":point.x*scale,"y":point.y*scale,"down":down ? 1<<16 : 0], binding:binding)
    }
    override func touchesBegan(_ touches:Set<UITouch>,with event:UIEvent?) {input(touches,down:true)}
    override func touchesMoved(_ touches:Set<UITouch>,with event:UIEvent?) {input(touches,down:true)}
    override func touchesEnded(_ touches:Set<UITouch>,with event:UIEvent?) {input(touches,down:false)}
    override func touchesCancelled(_ touches:Set<UITouch>,with event:UIEvent?) {input(touches,down:false)}
}
#endif
