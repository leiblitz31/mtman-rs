package com.mtman;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;

/**
 * Diagnostics screen. Every Rust binding is called in sequence and the raw
 * JSON (or error) is shown on screen — this is the on-device end-to-end check.
 */
public class Probe extends Activity {
    static { System.loadLibrary("mtman"); }

    private final StringBuilder sb = new StringBuilder();

    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        TextView tv = new TextView(this);
        tv.setTextSize(11);
        tv.setPadding(24, 24, 24, 24);
        setContentView(tv);

        String sdcard = getExternalFilesDir(null) != null
                ? getExternalFilesDir(null).getAbsolutePath() : "/sdcard";
        String apk = getPackageCodePath();

        line(tv, "=== greeting ===");
        try { line(tv, greeting()); } catch (Throwable t) { line(tv, "ERR " + t); }

        line(tv, "=== listDir " + sdcard + " ===");
        try { line(tv, listDir(sdcard)); } catch (Throwable t) { line(tv, "ERR " + t); }

        line(tv, "=== listDir /sdcard ===");
        try { line(tv, listDir("/sdcard")); } catch (Throwable t) { line(tv, "ERR " + t); }

        line(tv, "=== apkInfo (self) ===");
        try { line(tv, apkInfo(apk)); } catch (Throwable t) { line(tv, "ERR " + t); }

        java.io.File dex = asset("classes.dex");
        line(tv, "=== dexInfo ===");
        if (dex != null) {
            try { line(tv, dexInfo(dex.getAbsolutePath())); } catch (Throwable t) { line(tv, "ERR " + t); }
        } else {
            line(tv, "classes.dex not found in " + getExternalFilesDir(null));
        }

        java.io.File arsc = asset("resources.arsc");
        line(tv, "=== arscInfo ===");
        if (arsc != null) {
            try { line(tv, arscInfo(arsc.getAbsolutePath())); } catch (Throwable t) { line(tv, "ERR " + t); }
        } else {
            line(tv, "resources.arsc not found in " + getExternalFilesDir(null));
        }

        line(tv, "=== done ===");
        sb.insert(0, "MTMAN PROBE\n");
        tv.setText(sb.toString());
    }

    private void line(TextView tv, String s) {
        sb.append(s).append('\n');
        tv.setText(sb.toString());
    }

    /** Test inputs are pushed to the app's external files dir before launch. */
    private java.io.File asset(String name) {
        java.io.File dir = getExternalFilesDir(null);
        java.io.File f = new java.io.File(dir, name);
        return f.isFile() ? f : null;
    }

    // Bindings live in Jni; these wrappers keep the probe body readable.
    private static String greeting() { return Jni.greeting(); }
    private static String listDir(String p) { return Jni.listDir(p); }
    private static String apkInfo(String p) { return Jni.apkInfo(p); }
    private static String dexInfo(String p) { return Jni.dexInfo(p); }
    private static String arscInfo(String p) { return Jni.arscInfo(p); }
}
