package dev.raster.plugin.picker

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle

/**
 * Transparent helper Activity for startActivityForResult from NativeActivity.
 */
class RasterPickerActivity : Activity() {
    companion object {
        private const val REQUEST_CODE = 9102
        internal var pendingIntent: Intent? = null
        private var resultCallback: ((List<String>?) -> Unit)? = null

        fun launch(activity: Activity, intent: Intent, callback: (List<String>?) -> Unit) {
            pendingIntent = intent
            resultCallback = callback
            val proxy = Intent(activity, RasterPickerActivity::class.java)
            proxy.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            activity.startActivity(proxy)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val intent = pendingIntent
        if (intent == null) {
            deliverResult(null)
            finish()
            return
        }
        try {
            startActivityForResult(intent, REQUEST_CODE)
        } catch (_: Exception) {
            deliverResult(null)
            finish()
        }
    }

    @Deprecated("Deprecated in Java")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != REQUEST_CODE) {
            return
        }
        val uris = if (resultCode == RESULT_OK && data != null) extractUris(data) else null
        deliverResult(uris)
        finish()
    }

    override fun onBackPressed() {
        deliverResult(null)
        super.onBackPressed()
    }

    private fun deliverResult(uris: List<String>?) {
        pendingIntent = null
        val callback = resultCallback
        resultCallback = null
        callback?.invoke(uris)
    }

    private fun extractUris(data: Intent): List<String> {
        val uris = mutableListOf<String>()
        val clipData = data.clipData
        if (clipData != null) {
            for (index in 0 until clipData.itemCount) {
                val uri = clipData.getItemAt(index).uri
                if (uri != null) {
                    uris.add(uri.toString())
                }
            }
            return uris
        }
        val uri = data.data
        if (uri != null) {
            uris.add(uri.toString())
        }
        return uris
    }
}