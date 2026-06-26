package dev.raster.plugin.camera

import android.Manifest
import android.content.pm.PackageManager
import android.graphics.BitmapFactory
import android.net.Uri
import android.os.Build
import androidx.core.content.ContextCompat
import dev.raster.plugin.RasterPlugin
import dev.raster.plugin.picker.RasterImagePicker
import org.json.JSONObject

object CameraPlugin {
    @JvmStatic
    fun register() {
        RasterPlugin.register("Camera", "checkPermissions") { call ->
            call.replyOk(permissionStatus())
        }

        RasterPlugin.register("Camera", "requestPermissions") { call ->
            val activity = RasterPlugin.currentActivity()
            if (activity == null) {
                call.replyErr("NO_ACTIVITY", "No activity")
                return@register
            }
            activity.requestPermissions(requiredPermissions().toTypedArray(), 9101)
            call.replyOk(permissionStatus())
        }

        RasterPlugin.register("Camera", "takePhoto") { call ->
            val activity = RasterPlugin.currentActivity()
            if (activity == null) {
                call.replyErr("NO_ACTIVITY", "No activity")
                return@register
            }
            RasterImagePicker.takePhoto(activity) { uri ->
                if (uri == null) {
                    call.replyErr("USER_CANCELLED", "User cancelled camera")
                    return@takePhoto
                }
                call.replyOk(photoResultFromUri(activity, uri, "jpeg"))
            }
        }

        RasterPlugin.register("Camera", "pickImage") { call ->
            val activity = RasterPlugin.currentActivity()
            if (activity == null) {
                call.replyErr("NO_ACTIVITY", "No activity")
                return@register
            }
            RasterImagePicker.pickImage(activity) { uri ->
                if (uri == null) {
                    call.replyErr("USER_CANCELLED", "User cancelled picker")
                    return@pickImage
                }
                call.replyOk(photoResultFromUri(activity, uri, detectFormat(uri)))
            }
        }
    }

    private fun photoResultFromUri(
        activity: android.app.Activity,
        uri: Uri,
        format: String
    ): JSONObject {
        val resolver = activity.contentResolver
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        resolver.openInputStream(uri)?.use { stream ->
            BitmapFactory.decodeStream(stream, null, bounds)
        }
        return JSONObject()
            .put("uri", uri.toString())
            .put("width", bounds.outWidth)
            .put("height", bounds.outHeight)
            .put("format", format)
    }

    private fun detectFormat(uri: Uri): String {
        val path = uri.toString().lowercase()
        return when {
            path.endsWith(".png") -> "png"
            path.endsWith(".webp") -> "webp"
            else -> "jpeg"
        }
    }

    private fun requiredPermissions(): List<String> {
        val permissions = mutableListOf(Manifest.permission.CAMERA)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            permissions += Manifest.permission.READ_MEDIA_IMAGES
        }
        return permissions
    }

    private fun permissionStatus(): JSONObject {
        val activity = RasterPlugin.currentActivity()
        val camera = if (activity != null && ContextCompat.checkSelfPermission(
                activity,
                Manifest.permission.CAMERA
            ) == PackageManager.PERMISSION_GRANTED
        ) {
            "granted"
        } else {
            "prompt"
        }
        return JSONObject().put("camera", camera).put("photos", "prompt")
    }
}