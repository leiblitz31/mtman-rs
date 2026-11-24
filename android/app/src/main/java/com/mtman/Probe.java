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
        try { line(tv, nativeGreeting()); } catch (Throwable t) { line(tv, "ERR " + t); }

        line(tv, "=== listDir " + sdcard + " ===");
        try { line(tv, nativeListDir(sdcard)); } catch (Throwable t) { line(tv, "ERR " + t); }

        line(tv, "=== listDir /sdcard ===");
        try { line(tv, nativeListDir("/sdcard")); } catch (Throwable t) { line(tv, "ERR " + t); }

        line(tv, "=== apkInfo (self) ===");
        try { line(tv, nativeApkInfo(apk)); } catch (Throwable t) { line(tv, "ERR " + t); }

        java.io.File dex = asset("classes.dex");
        line(tv, "=== dexInfo ===");
        if (dex != null) {
            try { line(tv, nativeDexInfo(dex.getAbsolutePath())); } catch (Throwable t) { line(tv, "ERR " + t); }
        } else {
            line(tv, "classes.dex not found in " + getExternalFilesDir(null));
        }

        java.io.File arsc = asset("resources.arsc");
        line(tv, "=== arscInfo ===");
        if (arsc != null) {
            try { line(tv, nativeArscInfo(arsc.getAbsolutePath())); } catch (Throwable t) { line(tv, "ERR " + t); }
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

    public native String nativeGreeting();
    public native String nativeListDir(String path);
    public native String nativeApkInfo(String path);
    public native String nativeDexInfo(String path);
    public native String nativeArscInfo(String path);
}
