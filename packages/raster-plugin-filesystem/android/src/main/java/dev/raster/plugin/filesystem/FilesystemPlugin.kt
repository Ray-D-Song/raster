package dev.raster.plugin.filesystem

import android.net.Uri
import dev.raster.plugin.RasterPlugin
import org.json.JSONObject
import java.io.File

object FilesystemPlugin {
    @JvmStatic
    fun register() {
        RasterPlugin.register("Filesystem", "getCacheDirectory") { call ->
            val activity = RasterPlugin.currentActivity()
            if (activity == null) {
                call.replyErr("NO_ACTIVITY", "No activity")
                return@register
            }
            val uri = Uri.fromFile(activity.cacheDir).toString()
            call.replyOk(JSONObject().put("uri", uri))
        }

        RasterPlugin.register("Filesystem", "readText") { call ->
            val activity = RasterPlugin.currentActivity()
            if (activity == null) {
                call.replyErr("NO_ACTIVITY", "No activity")
                return@register
            }
            val uri = call.args?.optString("uri")
            if (uri.isNullOrBlank()) {
                call.replyErr("INVALID_ARGS", "uri is required")
                return@register
            }
            val file = resolveScopedFile(activity.cacheDir, uri)
            if (file == null || !file.exists()) {
                call.replyErr("READ_FAILED", "File not found or out of scope")
                return@register
            }
            runCatching {
                call.replyOk(JSONObject().put("text", file.readText()))
            }.onFailure {
                call.replyErr("READ_FAILED", it.message ?: "read failed")
            }
        }

        RasterPlugin.register("Filesystem", "writeText") { call ->
            val activity = RasterPlugin.currentActivity()
            if (activity == null) {
                call.replyErr("NO_ACTIVITY", "No activity")
                return@register
            }
            val uri = call.args?.optString("uri")
            val text = call.args?.optString("text")
            if (uri.isNullOrBlank() || text == null) {
                call.replyErr("INVALID_ARGS", "uri and text are required")
                return@register
            }
            val file = resolveScopedFile(activity.cacheDir, uri)
                ?: File(activity.cacheDir, "raster-${System.currentTimeMillis()}.txt")
            runCatching {
                file.parentFile?.mkdirs()
                file.writeText(text)
                call.replyOk(JSONObject().put("uri", Uri.fromFile(file).toString()))
            }.onFailure {
                call.replyErr("WRITE_FAILED", it.message ?: "write failed")
            }
        }
    }

    private fun resolveScopedFile(cacheDir: File, uri: String): File? {
        val parsed = Uri.parse(uri)
        val path = parsed.path ?: return null
        val file = File(path)
        val cachePath = cacheDir.canonicalPath
        val targetPath = file.canonicalPath
        return if (targetPath.startsWith(cachePath)) file else null
    }
}