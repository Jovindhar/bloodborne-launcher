// Draws the app icon into an .iconset folder: the source of the launcher's icons (src-tauri/icons).
// a pale crescent moon with a soft glow over a blue-slate night (the launcher's colours, src/index.css),
// on the macOS rounded-square shape.
//   swift scripts/make_icon.swift OUT.iconset [--full-bleed]
// --full-bleed: the whole square, no margin or corners (iOS, which rounds icons itself).
// then: npx tauri icon OUT.iconset/icon_512x512@2x.png (src-tauri/icons; keep the macOS ones).
import AppKit

let fullBleed = CommandLine.arguments.contains("--full-bleed")

func render(_ size: Int) -> Data {
    let s = CGFloat(size)
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
                               bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let ctx = NSGraphicsContext.current!.cgContext

    // Body: the rounded square, inset as macOS app icons are.
    let inset = fullBleed ? 0 : s * 0.1
    let body = CGRect(x: inset, y: inset, width: s - 2 * inset, height: s - 2 * inset)
    let corner = fullBleed ? 0 : body.width * 0.225
    let shape = CGPath(roundedRect: body, cornerWidth: corner, cornerHeight: corner, transform: nil)
    ctx.saveGState()
    ctx.addPath(shape)
    ctx.clip()
    // Slate: lighter behind the moon (#2a3242), the night at the edges (#07090d).
    let colors = [CGColor(red: 0.165, green: 0.196, blue: 0.259, alpha: 1),
                  CGColor(red: 0.027, green: 0.035, blue: 0.051, alpha: 1)] as CFArray
    let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colors,
                              locations: [0, 1])!
    ctx.drawRadialGradient(gradient, startCenter: CGPoint(x: body.midX, y: body.midY + body.height * 0.15),
                           startRadius: 0, endCenter: CGPoint(x: body.midX, y: body.midY),
                           endRadius: body.width * 0.75, options: .drawsAfterEndLocation)
    // The moon: a disc with a darker disc cut out of it.
    let r = body.width * 0.27
    let center = CGPoint(x: body.midX + r * 0.1, y: body.midY + r * 0.15)
    let moon = CGRect(x: center.x - r, y: center.y - r, width: 2 * r, height: 2 * r)
    let bite = CGRect(x: center.x - r * 0.55, y: center.y - r * 0.6, width: 2 * r * 0.95,
                      height: 2 * r * 0.95)
    // A faint glow fading out from the disc, with no edge, then the crescent (the disc minus the bite).
    let glow = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                          colors: [CGColor(red: 0.93, green: 0.89, blue: 0.83, alpha: 0.16),
                                   CGColor(red: 0.93, green: 0.89, blue: 0.83, alpha: 0)] as CFArray,
                          locations: [0, 1])!
    ctx.drawRadialGradient(glow, startCenter: center, startRadius: r, endCenter: center,
                           endRadius: r * 1.6, options: [])
    ctx.addEllipse(in: moon)
    ctx.clip()
    ctx.addEllipse(in: moon)
    ctx.addEllipse(in: bite)
    ctx.setFillColor(CGColor(red: 0.93, green: 0.89, blue: 0.83, alpha: 1)) // parchment, #ece4d4
    ctx.fillPath(using: .evenOdd)
    ctx.restoreGState()
    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])!
}

let out = URL(fileURLWithPath: CommandLine.arguments[1])
try FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
for base in [16, 32, 128, 256, 512] {
    try render(base).write(to: out.appendingPathComponent("icon_\(base)x\(base).png"))
    try render(base * 2).write(to: out.appendingPathComponent("icon_\(base)x\(base)@2x.png"))
}
