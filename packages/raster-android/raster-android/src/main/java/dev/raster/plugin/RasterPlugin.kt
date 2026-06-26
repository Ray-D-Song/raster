package dev.raster.plugin

import android.app.Activity
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap

object RasterPlugin {
    class Call internal constructor(
        internal val id: Long,
        internal val argsJson: String?
    ) {
        val args: JSONObject? = argsJson?.let { runCatching { JSONObject(it) }.getOrNull() }

        fun replyOk(result: JSONObject) {
            nativeReplyOk(id, result.toString())
        }

        fun replyErr(code: String, message: String) {
            nativeReplyErr(id, code, message)
        }
    }

    private val handlers = ConcurrentHashMap<String, (Call) -> Unit>()

    fun register(plugin: String, method: String, handler: (Call) -> Unit) {
        handlers["$plugin:$method"] = handler
        nativeRegister(plugin, method)
    }

    @JvmStatic
    fun dispatchFromNative(callId: Long, plugin: String, method: String, argsJson: String?) {
        val activity = currentActivity()
        if (activity != null) {
            activity.runOnUiThread {
                dispatchOnMainThread(callId, plugin, method, argsJson)
            }
            return
        }
        dispatchOnMainThread(callId, plugin, method, argsJson)
    }

    private fun dispatchOnMainThread(
        callId: Long,
        plugin: String,
        method: String,
        argsJson: String?
    ) {
        val handler = handlers["$plugin:$method"]
        if (handler == null) {
            nativeReplyErr(callId, "UNIMPLEMENTED", "No handler for $plugin.$method")
            return
        }
        handler(Call(callId, argsJson))
    }

    fun currentActivity(): Activity? {
        val pointer = nativeCurrentActivity()
        return if (pointer == 0L) null else ActivityHolder.fromPointer(pointer)
    }

    fun emit(plugin: String, event: String, data: JSONObject = JSONObject()) {
        nativeEmitEvent(plugin, event, data.toString())
    }

    private object ActivityHolder {
        private var activity: Activity? = null

        fun bind(activity: Activity) {
            this.activity = activity
        }

        fun fromPointer(@Suppress("UNUSED_PARAMETER") pointer: Long): Activity? = activity
    }

    fun bindActivity(activity: Activity) {
        ActivityHolder.bind(activity)
        nativeSetCurrentActivity(activity)
    }

    @JvmStatic
    private external fun nativeRegister(plugin: String, method: String)

    @JvmStatic
    private external fun nativeReplyOk(callId: Long, resultJson: String)

    @JvmStatic
    private external fun nativeReplyErr(callId: Long, code: String, message: String)

    @JvmStatic
    private external fun nativeEmitEvent(plugin: String, event: String, dataJson: String)

    @JvmStatic
    private external fun nativeCurrentActivity(): Long

    @JvmStatic
    private external fun nativeSetCurrentActivity(activity: Activity)
}