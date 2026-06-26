import AVFoundation
import Photos
import RasterPluginSDK
import UIKit

public enum CameraPlugin {
    private static var pickerDelegate: PickerDelegate?

    public static func register() {
        RasterPlugin.register(plugin: "Camera", method: "checkPermissions") { call in
            call.replyOk(permissionPayload())
        }

        RasterPlugin.register(plugin: "Camera", method: "requestPermissions") { call in
            requestPermissions { call.replyOk(permissionPayload()) }
        }

        RasterPlugin.register(plugin: "Camera", method: "takePhoto") { call in
            let quality = (call.args?["quality"] as? NSNumber)?.intValue ?? 90
            presentPicker(source: .camera, quality: quality, call: call)
        }

        RasterPlugin.register(plugin: "Camera", method: "pickImage") { call in
            let quality = (call.args?["quality"] as? NSNumber)?.intValue ?? 90
            presentPicker(source: .photoLibrary, quality: quality, call: call)
        }
    }

    private static func permissionPayload() -> [String: Any] {
        [
            "camera": authStatus(for: .video),
            "photos": photosAuthStatus(),
        ]
    }

    private static func authStatus(for mediaType: AVMediaType) -> String {
        switch AVCaptureDevice.authorizationStatus(for: mediaType) {
        case .authorized: return "granted"
        case .denied, .restricted: return "denied"
        case .notDetermined: return "prompt"
        @unknown default: return "prompt"
        }
    }

    private static func photosAuthStatus() -> String {
        switch PHPhotoLibrary.authorizationStatus(for: .readWrite) {
        case .authorized, .limited: return "granted"
        case .denied, .restricted: return "denied"
        case .notDetermined: return "prompt"
        @unknown default: return "prompt"
        }
    }

    private static func requestPermissions(completion: @escaping () -> Void) {
        let group = DispatchGroup()
        group.enter()
        AVCaptureDevice.requestAccess(for: .video) { _ in group.leave() }
        group.enter()
        PHPhotoLibrary.requestAuthorization(for: .readWrite) { _ in group.leave() }
        group.notify(queue: .main, execute: completion)
    }

    private static func presentPicker(
        source: UIImagePickerController.SourceType,
        quality: Int,
        call: RasterPlugin.Call
    ) {
        DispatchQueue.main.async {
            guard let root = RasterPlugin.rootViewController() else {
                call.replyErr(code: "NO_UI", message: "No root view controller")
                return
            }
            guard UIImagePickerController.isSourceTypeAvailable(source) else {
                call.replyErr(code: "UNAVAILABLE", message: "Image source unavailable")
                return
            }
            let picker = UIImagePickerController()
            picker.sourceType = source
            picker.allowsEditing = false
            let delegate = PickerDelegate(quality: quality, call: call)
            pickerDelegate = delegate
            picker.delegate = delegate
            root.present(picker, animated: true)
        }
    }
}

private final class PickerDelegate: NSObject, UIImagePickerControllerDelegate, UINavigationControllerDelegate {
    private let quality: Int
    private let call: RasterPlugin.Call

    init(quality: Int, call: RasterPlugin.Call) {
        self.quality = quality
        self.call = call
    }

    func imagePickerControllerDidCancel(_ picker: UIImagePickerController) {
        picker.dismiss(animated: true)
        call.replyErr(code: "USER_CANCELLED", message: "User cancelled image picker")
    }

    func imagePickerController(
        _ picker: UIImagePickerController,
        didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey: Any]
    ) {
        picker.dismiss(animated: true)
        guard let image = info[.originalImage] as? UIImage else {
            call.replyErr(code: "NO_IMAGE", message: "No image returned")
            return
        }
        let jpegQuality = CGFloat(max(1, min(quality, 100))) / 100.0
        guard let data = image.jpegData(compressionQuality: jpegQuality) else {
            call.replyErr(code: "ENCODE_FAILED", message: "Failed to encode JPEG")
            return
        }
        let fileName = "raster-camera-\(UUID().uuidString).jpg"
        let fileURL = FileManager.default.temporaryDirectory.appendingPathComponent(fileName)
        do {
            try data.write(to: fileURL, options: .atomic)
        } catch {
            call.replyErr(code: "WRITE_FAILED", message: error.localizedDescription)
            return
        }
        call.replyOk([
            "uri": fileURL.absoluteString,
            "width": Int(image.size.width.rounded()),
            "height": Int(image.size.height.rounded()),
            "format": "jpeg",
        ])
    }
}