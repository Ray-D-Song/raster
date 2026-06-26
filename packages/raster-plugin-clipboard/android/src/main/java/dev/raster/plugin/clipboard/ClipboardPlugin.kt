package dev.raster.plugin.clipboard

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import dev.raster.plugin.RasterPlugin
import org.json.JSONObject

object ClipboardPlugin {
    @JvmStatic
    fun register() {
        RasterPlugin.register("Clipboard", "getString") { call ->
            val activity = RasterPlugin.currentActivity()
            if (activity == null) {
                call.replyErr("NO_ACTIVITY", "No activity")
                return@register
            }
            val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            val item = clipboard.primaryClip?.getItemAt(0)
            val value = item?.coerceToText(activity)?.toString()
            call.replyOk(JSONObject().put("value", value))
        }

        RasterPlugin.register("Clipboard", "setString") { call ->
            val activity = RasterPlugin.currentActivity()
            if (activity == null) {
                call.replyErr("NO_ACTIVITY", "No activity")
                return@register
            }
            val value = call.args?.optString("value")
            if (value == null) {
                call.replyErr("INVALID_ARGS", "value is required")
                return@register
            }
            val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("raster", value))
            call.replyOk(JSONObject().put("ok", true))
        }
    }
}