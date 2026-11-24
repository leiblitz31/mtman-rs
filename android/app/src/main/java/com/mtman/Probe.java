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
        final TextView tv = new TextView(this);
        tv.setTextSize(11);
        tv.setPadding(24, 24, 24, 24);
        setContentView(tv);

        final String sdcard = getExternalFilesDir(null) != null
                ? getExternalFilesDir(null).getAbsolutePath() : "/sdcard";
        final String apk = getPackageCodePath();
        final java.io.File dex = asset("classes.dex");
        final java.io.File arsc = asset("resources.arsc");

        line(tv, "MTMAN PROBE — running...");
        // All calls block; keep them off the main thread or the UI ANRs.
        new Thread(new Runnable() {
            @Override
            public void run() {
                final StringBuilder out = new StringBuilder();
                step(out, "greeting", new Call() { public String get() { return greeting(); } });
                step(out, "listDir " + sdcard, new Call() { public String get() { return listDir(sdcard); } });
                step(out, "listDir /sdcard", new Call() { public String get() { return listDir("/sdcard"); } });
                step(out, "apkInfo (self)", new Call() { public String get() { return apkInfo(apk); } });
                if (dex != null) {
                    step(out, "dexInfo", new Call() { public String get() { return dexInfo(dex.getAbsolutePath()); } });
                } else {
                    out.append("dexInfo: not staged\n");
                }
                if (arsc != null) {
                    step(out, "arscInfo", new Call() { public String get() { return arscInfo(arsc.getAbsolutePath()); } });
                } else {
                    out.append("arscInfo: not staged\n");
                }
                out.append("=== done ===");

                runOnUiThread(new Runnable() {
                    @Override
                    public void run() {
                        tv.setText(out.toString());
                    }
                });
            }
        }).start();
    }

    private interface Call { String get(); }

    private void step(StringBuilder out, String label, Call c) {
        long t0 = System.currentTimeMillis();
        String r;
        try {
            r = c.get();
        } catch (Throwable t) {
            r = "ERR " + t;
        }
        long ms = System.currentTimeMillis() - t0;
        out.append("--- ").append(label).append(" (").append(ms).append(" ms)\n")
           .append(r).append('\n');
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
