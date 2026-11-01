import AVFoundation
import CoreGraphics
import Foundation
import ImageIO

let arguments = CommandLine.arguments
guard arguments.count == 5, let sourceFps = Int32(arguments[2]) else {
    FileHandle.standardError.write("usage: mp4_mux <frames-dir> <fps> <audio.m4a> <out.mp4>\n".data(using: .utf8)!)
    exit(2)
}
let framesDir = URL(fileURLWithPath: arguments[1])
let audioURL = URL(fileURLWithPath: arguments[3])
let outURL = URL(fileURLWithPath: arguments[4])
let videoOnlyURL = outURL.deletingPathExtension().appendingPathExtension("video.mp4")
let repeats: Int32 = sourceFps < 24 ? 2 : 1
let fps = sourceFps * repeats
let bitRate = 14_000_000

func load(_ name: String) -> CGImage {
    let source = CGImageSourceCreateWithURL(framesDir.appendingPathComponent(name) as CFURL, nil)!
    return CGImageSourceCreateImageAtIndex(source, 0, nil)!
}

func encodeFrames(_ names: [String]) async throws -> (Int, Int) {
    let first = load(names[0])
    let (width, height) = (first.width, first.height)
    try? FileManager.default.removeItem(at: videoOnlyURL)
    let writer = try AVAssetWriter(outputURL: videoOnlyURL, fileType: .mp4)
    let input = AVAssetWriterInput(mediaType: .video, outputSettings: [
        AVVideoCodecKey: AVVideoCodecType.h264,
        AVVideoWidthKey: width,
        AVVideoHeightKey: height,
        AVVideoColorPropertiesKey: [
            AVVideoColorPrimariesKey: AVVideoColorPrimaries_ITU_R_709_2,
            AVVideoTransferFunctionKey: AVVideoTransferFunction_ITU_R_709_2,
            AVVideoYCbCrMatrixKey: AVVideoYCbCrMatrix_ITU_R_709_2,
        ],
        AVVideoCompressionPropertiesKey: [
            AVVideoAverageBitRateKey: bitRate,
            AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel,
            AVVideoMaxKeyFrameIntervalKey: Int(fps) * 2,
        ],
    ])
    input.expectsMediaDataInRealTime = false
    let adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input, sourcePixelBufferAttributes: [
        kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
        kCVPixelBufferWidthKey as String: width,
        kCVPixelBufferHeightKey as String: height,
    ])
    writer.add(input)
    writer.startWriting()
    writer.startSession(atSourceTime: .zero)
    var index: Int64 = 0
    for name in names {
        let image = load(name)
        for _ in 0..<repeats {
            while !input.isReadyForMoreMediaData { try await Task.sleep(for: .milliseconds(2)) }
            var buffer: CVPixelBuffer?
            CVPixelBufferPoolCreatePixelBuffer(nil, adaptor.pixelBufferPool!, &buffer)
            let pixels = buffer!
            CVPixelBufferLockBaseAddress(pixels, [])
            let context = CGContext(
                data: CVPixelBufferGetBaseAddress(pixels), width: width, height: height, bitsPerComponent: 8,
                bytesPerRow: CVPixelBufferGetBytesPerRow(pixels), space: CGColorSpace(name: CGColorSpace.sRGB)!,
                bitmapInfo: CGImageAlphaInfo.noneSkipFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)!
            context.interpolationQuality = .none
            context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
            CVPixelBufferUnlockBaseAddress(pixels, [])
            adaptor.append(pixels, withPresentationTime: CMTime(value: index, timescale: fps))
            index += 1
        }
    }
    input.markAsFinished()
    await writer.finishWriting()
    guard writer.status == .completed else { throw writer.error ?? CocoaError(.fileWriteUnknown) }
    return (width, height)
}

func mux() async throws -> Double {
    let video = AVURLAsset(url: videoOnlyURL)
    let audio = AVURLAsset(url: audioURL)
    let duration = try await video.load(.duration)
    let audioDuration = CMTimeMinimum(duration, try await audio.load(.duration))
    let composition = AVMutableComposition()
    let videoTrack = composition.addMutableTrack(withMediaType: .video, preferredTrackID: kCMPersistentTrackID_Invalid)!
    try videoTrack.insertTimeRange(
        CMTimeRange(start: .zero, duration: duration), of: try await video.loadTracks(withMediaType: .video)[0], at: .zero)
    let audioTrack = composition.addMutableTrack(withMediaType: .audio, preferredTrackID: kCMPersistentTrackID_Invalid)!
    try audioTrack.insertTimeRange(
        CMTimeRange(start: .zero, duration: audioDuration), of: try await audio.loadTracks(withMediaType: .audio)[0], at: .zero)
    try? FileManager.default.removeItem(at: outURL)
    let session = AVAssetExportSession(asset: composition, presetName: AVAssetExportPresetPassthrough)!
    session.shouldOptimizeForNetworkUse = true
    try await session.export(to: outURL, as: .mp4)
    try? FileManager.default.removeItem(at: videoOnlyURL)
    return CMTimeGetSeconds(duration)
}

let names = try FileManager.default.contentsOfDirectory(atPath: framesDir.path).filter { $0.hasSuffix(".png") }.sorted()
let (width, height) = try await encodeFrames(names)
let seconds = try await mux()
print("wrote \(outURL.path): \(width)x\(height), \(names.count) frames shown at \(fps) fps, \(seconds) s")
