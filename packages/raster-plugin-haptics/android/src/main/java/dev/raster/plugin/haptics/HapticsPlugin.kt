package dev.raster.plugin.haptics

import android.os.Build
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import dev.raster.plugin.RasterPlugin
import org.json.JSONObject

object HapticsPlugin {
    @JvmStatic
    fun register() {
        RasterPlugin.register("Haptics", "impact") { call ->
            val activity = RasterPlugin.currentActivity()
            if (activity == null) {
                call.replyErr("NO_ACTIVITY", "No activity")
                return@register
            }
            val style = call.args?.optString("style", "medium") ?: "medium"
            val duration = when (style) {
                "light" -> 20L
                "heavy" -> 60L
                else -> 40L
            }
            vibrate(activity, duration)
            call.replyOk(JSONObject().put("ok", true))
        }

        RasterPlugin.register("Haptics", "vibrate") { call ->
            val activity = RasterPlugin.currentActivity()
            if (activity == null) {
                call.replyErr("NO_ACTIVITY", "No activity")
                return@register
            }
            val duration = call.args?.optLong("duration", 50L) ?: 50L
            vibrate(activity, duration.coerceIn(1L, 500L))
            call.replyOk(JSONObject().put("ok", true))
        }
    }

    private fun vibrate(activity: android.app.Activity, durationMs: Long) {
        val vibrator = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            val manager = activity.getSystemService(VibratorManager::class.java)
            manager?.defaultVibrator
        } else {
            @Suppress("DEPRECATION")
            activity.getSystemService(Vibrator::class.java)
        } ?: return

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            vibrator.vibrate(VibrationEffect.createOneShot(durationMs, VibrationEffect.DEFAULT_AMPLITUDE))
        } else {
            @Suppress("DEPRECATION")
            vibrator.vibrate(durationMs)
        }
    }
}