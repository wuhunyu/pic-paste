// PicPasteOCR — macOS 14+ offline Apple Vision helper.
// Deliberately a separate one-shot process: no network, no listening socket,
// no filesystem input, no application-wide initialization until explicitly used.
import Foundation
import CoreGraphics
import CryptoKit
import Vision

private let protocolVersion: UInt32 = 1
private let maxRGBA = 80 * 1024 * 1024
private let maxJSON = 8 * 1024 * 1024

private struct OcrWord: Encodable {
    let text: String
    let confidence: Double
    let polygon_px: [[Double]]
}

private struct OcrLine: Encodable {
    let text: String
    let confidence: Double
    let polygon_px: [[Double]]
    let words: [OcrWord]
}

private struct OcrDocument: Encodable {
    let image_hash: String
    let pixel_width: UInt32
    let pixel_height: UInt32
    let provider: String
    let model_revision: String
    let lines: [OcrLine]
}

private struct Envelope: Encodable {
    let protocol_version: UInt32
    let document: OcrDocument?
    let error: String?
}

private enum Failure: Error, LocalizedError {
    case invalid(String)
    var errorDescription: String? {
        switch self {
        case .invalid(let message): return message
        }
    }
}

private func readExact(_ size: Int) throws -> Data {
    guard size >= 0 && size <= maxRGBA else {
        throw Failure.invalid("input size exceeds 80 MiB")
    }
    var bytes = Data()
    while bytes.count < size {
        let part = FileHandle.standardInput.readData(ofLength: size - bytes.count)
        guard !part.isEmpty else { throw Failure.invalid("truncated OCR input") }
        bytes.append(part)
    }
    return bytes
}

private func u32le(_ bytes: [UInt8], at offset: Int) -> UInt32 {
    UInt32(bytes[offset]) |
      (UInt32(bytes[offset + 1]) << 8) |
      (UInt32(bytes[offset + 2]) << 16) |
      (UInt32(bytes[offset + 3]) << 24)
}

private func encode(_ envelope: Envelope) throws -> Data {
    let data = try JSONEncoder().encode(envelope)
    guard data.count > 0 && data.count <= maxJSON else {
        throw Failure.invalid("OCR output exceeds 8 MiB")
    }
    var header = UInt32(data.count).littleEndian
    var framed = withUnsafeBytes(of: &header) { Data($0) }
    framed.append(data)
    return framed
}

private func polygon(_ rect: CGRect, width: UInt32, height: UInt32) -> [[Double]] {
    let w = Double(width)
    let h = Double(height)
    let x0 = (Double(rect.minX) * w).clamped(to: 0...w)
    let x1 = (Double(rect.maxX) * w).clamped(to: 0...w)
    let y0 = ((1.0 - Double(rect.maxY)) * h).clamped(to: 0...h)
    let y1 = ((1.0 - Double(rect.minY)) * h).clamped(to: 0...h)
    return [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

private extension Double {
    func clamped(to bounds: ClosedRange<Double>) -> Double {
        min(max(self, bounds.lowerBound), bounds.upperBound)
    }
}

private func recognize() throws -> OcrDocument {
    let header = Array(try readExact(16))
    guard Array(header[0..<4]) == Array("SHOC".utf8) else {
        throw Failure.invalid("bad OCR protocol magic")
    }
    let version = u32le(header, at: 4)
    let width = u32le(header, at: 8)
    let height = u32le(header, at: 12)
    guard version == protocolVersion, width > 0, height > 0 else {
        throw Failure.invalid("unsupported OCR protocol version or empty image")
    }
    let (pixelCount, pixelOverflow) = Int(width).multipliedReportingOverflow(by: Int(height))
    let (byteCount, byteOverflow) = pixelCount.multipliedReportingOverflow(by: 4)
    guard !pixelOverflow, !byteOverflow, byteCount > 0, byteCount <= maxRGBA else {
        throw Failure.invalid("invalid OCR RGBA dimensions")
    }
    let rgba = try readExact(byteCount)
    let hash = SHA256.hash(data: rgba)
        .map { String(format: "%02x", $0) }
        .joined()

    guard let provider = CGDataProvider(data: rgba as CFData) else {
        throw Failure.invalid("cannot create RGBA image provider")
    }
    let bitmapInfo = CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue)
        .union(CGBitmapInfo.byteOrder32Big)
    guard let image = CGImage(
        width: Int(width), height: Int(height),
        bitsPerComponent: 8, bitsPerPixel: 32,
        bytesPerRow: Int(width) * 4,
        space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: bitmapInfo,
        provider: provider, decode: nil,
        shouldInterpolate: false, intent: .defaultIntent
    ) else {
        throw Failure.invalid("cannot create CGImage")
    }
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.usesLanguageCorrection = true
    request.recognitionLanguages = ["zh-Hans", "en-US"]
    let handler = VNImageRequestHandler(cgImage: image, options: [:])
    try handler.perform([request])

    let lines = (request.results ?? []).compactMap { observation -> OcrLine? in
        guard let best = observation.topCandidates(1).first else { return nil }
        return OcrLine(
            text: best.string,
            confidence: Double(best.confidence).clamped(to: 0...1),
            polygon_px: polygon(observation.boundingBox, width: width, height: height),
            words: []
        )
    }
    return OcrDocument(
        image_hash: hash,
        pixel_width: width,
        pixel_height: height,
        provider: "apple-vision",
        model_revision: ProcessInfo.processInfo.operatingSystemVersionString,
        lines: lines
    )
}

do {
    let document = try recognize()
    FileHandle.standardOutput.write(try encode(Envelope(
        protocol_version: protocolVersion,
        document: document,
        error: nil
    )))
} catch {
    let message = String(describing: error)
    do {
        FileHandle.standardOutput.write(try encode(Envelope(
            protocol_version: protocolVersion,
            document: nil,
            error: message
        )))
    } catch {
        fputs("PicPasteOCR could not encode its error response\n", stderr)
    }
    exit(EXIT_FAILURE)
}
