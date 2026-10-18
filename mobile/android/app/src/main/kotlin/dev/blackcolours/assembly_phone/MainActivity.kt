package dev.blackcolours.assembly_phone

import android.view.inputmethod.InputMethodManager
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugins.webviewflutter.WebViewFlutterAndroidExternalApi

class MainActivity : FlutterActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        // The terminal's Keyboard key. A WebView shows the keyboard for a page's focus() only after
        // a tap inside it, so give the WebView Android focus and show the keyboard here.
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "assembly/terminal")
            .setMethodCallHandler { call, result ->
                if (call.method != "showKeyboard") return@setMethodCallHandler result.notImplemented()
                @Suppress("DEPRECATION")
                val webView = WebViewFlutterAndroidExternalApi.getWebView(
                    flutterEngine, (call.arguments as Number).toLong(),
                ) ?: return@setMethodCallHandler result.success(false)
                webView.requestFocus()
                // After the focus change has reached the input method.
                webView.post {
                    getSystemService(InputMethodManager::class.java).showSoftInput(webView, 0)
                }
                result.success(true)
            }
    }
}
