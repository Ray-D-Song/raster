package dev.raster.plugin.picker

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.MediaStore
import androidx.core.content.FileProvider
import java.io.File

object RasterImagePicker {
    fun takePhoto(activity: Activity, callback: (Uri?) -> Unit) {
        val photoFile = File(activity.cacheDir, "raster-camera-${System.currentTimeMillis()}.jpg")
        val uri = FileProvider.getUriForFile(
            activity,
            "${activity.packageName}.rasterfileprovider",
            photoFile
        )
        val intent = Intent(MediaStore.ACTION_IMAGE_CAPTURE).apply {
            putExtra(MediaStore.EXTRA_OUTPUT, uri)
            addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        RasterPickerActivity.launch(activity, intent) { _ ->
            callback(if (photoFile.exists() && photoFile.length() > 0) uri else null)
        }
    }

    fun pickImage(activity: Activity, callback: (Uri?) -> Unit) {
        val intent = Intent(Intent.ACTION_PICK, MediaStore.Images.Media.EXTERNAL_CONTENT_URI).apply {
            type = "image/*"
        }
        RasterPickerActivity.launch(activity, intent) { uris ->
            val uri = uris?.firstOrNull()?.let(Uri::parse)
            callback(uri)
        }
    }
}