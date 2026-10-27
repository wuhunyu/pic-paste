// Deterministic native QR fixture for the macOS Vision helper smoke test.
// Emits a complete SHOC v1 RGBA request to stdout; never touches the network.
import CoreGraphics
import CoreImage
import Foundation

let payload = "PICPASTE-QR-OFFLINE-TEST"
guard let filter = CIFilter(name: "CIQRCodeGenerator") else {
    fatalError("CoreImage QR generator is missing")
}
filter.setValue(Data(payload.utf8), forKey: "inputMessage")
filter.setValue("Q", forKey: "inputCorrectionLevel")
guard let image = filter.outputImage,
      let source = CIContext().createCGImage(image, from: image.extent) else {
    fatalError("Could not render deterministic QR code")
}

let size = 360
var pixels = [UInt8](repeating: 255, count: size * size * 4)
let rendered = pixels.withUnsafeMutableBytes { storage -> Bool in
    let bitmap = CGImageAlphaInfo.premultipliedLast.rawValue
        | CGBitmapInfo.byteOrder32Big.rawValue
    guard let context = CGContext(
        data: storage.baseAddress,
        width: size,
        height: size,
        bitsPerComponent: 8,
        bytesPerRow: size * 4,
        space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: bitmap
    ) else {
        return false
    }
    context.interpolationQuality = .none
    let side = CGFloat(source.width * 8)
    let origin = (CGFloat(size) - side) / 2
    context.draw(source, in: CGRect(x: origin, y: origin, width: side, height: side))
    return true
}
guard rendered else {
    fatalError("Cannot draw QR code into RGBA")
}

var output = Data("SHOC".utf8)
for number in [UInt32(1), UInt32(size), UInt32(size)] {
    var littleEndian = number.littleEndian
    output.append(Data(bytes: &littleEndian, count: MemoryLayout<UInt32>.size))
}
output.append(contentsOf: pixels)
FileHandle.standardOutput.write(output)
