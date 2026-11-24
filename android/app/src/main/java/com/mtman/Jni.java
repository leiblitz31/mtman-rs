package com.mtman;

/**
 * The single JNI entry point. Native method names are bound to this class
 * (Java_com_mtman_Jni_...), so every other class must route through here —
 * putting a native method on another class gives UnsatisfiedLinkError at call.
 *
 * Methods are static: stateless, and callable without an instance.
 */
public final class Jni {
    static { System.loadLibrary("mtman"); }

    private Jni() {}

    public static native String greeting();
    public static native String listDir(String path);
    public static native String apkInfo(String path);
    public static native String dexInfo(String path);
    public static native String arscInfo(String path);
}
