//! MT Manager — Rust rewrite.
//!
//! Modules:
//!   - `fs`       — file operations (stat, read, write, list)
//!   - `apk`      — APK/ZIP parser (central dir, stored + deflate)
//!   - `dex`      — DEX parser (header, strings, classes, methods)
//!   - `arsc`     — Android resource table parser

pub mod apk;
pub mod arsc;
pub mod dex;
pub mod fs;

// ---------------------------------------------------------------------------
// JNI surface. Everything returns JSON so the Java side only needs org.json,
// which ships with Android — no extra dependency and no object marshalling.
// ---------------------------------------------------------------------------

#[cfg(all(target_os = "android", feature = "android"))]
mod jni_api {
    use super::{apk, arsc, dex, fs};
    use jni::JNIEnv;
    use jni::objects::{JClass, JString};
    use serde_json::json;
    use std::path::Path;

    fn to_jstring<'a>(env: &JNIEnv<'a>, s: String) -> JString<'a> {
        env.new_string(s).expect("OOM creating JString")
    }

    /// `nativeListDir(String path) -> String` (JSON array).
    #[no_mangle]
    pub extern "system" fn Java_com_mtman_Jni_listDir<'a>(
        mut env: JNIEnv<'a>,
        _class: JClass<'a>,
        path: JString<'a>,
    ) -> JString<'a> {
        let path: String = env.get_string(&path).unwrap().into();
        let items = match fs::list_dir(Path::new(&path)) {
            Ok(entries) => entries
                .into_iter()
                .map(|e| {
                    json!({
                        "name": e.name,
                        "dir": e.is_dir,
                        "size": e.size,
                        "mtime": e.mtime,
                    })
                })
                .collect::<Vec<_>>(),
            Err(e) => vec![json!({"error": e.to_string()})],
        };
        to_jstring(&env, serde_json::to_string(&items).unwrap())
    }

    /// `nativeApkInfo(String path) -> String` (JSON).
    #[no_mangle]
    pub extern "system" fn Java_com_mtman_Jni_apkInfo<'a>(
        mut env: JNIEnv<'a>,
        _class: JClass<'a>,
        path: JString<'a>,
    ) -> JString<'a> {
        let path: String = env.get_string(&path).unwrap().into();
        let out = match apk::Apk::open(Path::new(&path)) {
            Ok(apk) => {
                let entries = apk.entries.len();
                // Top-level names inside the APK, for a quick overview.
                let mut top: Vec<String> = apk
                    .entries
                    .keys()
                    .filter_map(|n| n.split('/').next().map(|s| s.to_string()))
                    .collect();
                top.sort();
                top.dedup();
                json!({"entries": entries, "top_entries": top})
            }
            Err(e) => json!({"error": e.to_string()}),
        };
        to_jstring(&env, serde_json::to_string(&out).unwrap())
    }

    /// `nativeDexInfo(String path) -> String` (JSON).
    #[no_mangle]
    pub extern "system" fn Java_com_mtman_Jni_dexInfo<'a>(
        mut env: JNIEnv<'a>,
        _class: JClass<'a>,
        path: JString<'a>,
    ) -> JString<'a> {
        let path: String = env.get_string(&path).unwrap().into();
        let out = match fs::read_file(Path::new(&path)) {
            Ok(data) => match dex::Dex::parse(&data) {
                Ok(d) => {
                    let classes = d.class_names().unwrap_or_default();
                    let sample: Vec<String> = classes.iter().take(20).cloned().collect();
                    json!({
                        "strings": d.header.string_ids_size,
                        "types": d.header.type_ids_size,
                        "methods": d.header.method_ids_size,
                        "classes": d.header.class_defs_size,
                        "class_sample": sample,
                    })
                }
                Err(e) => json!({"error": format!("{e}")}),
            },
            Err(e) => json!({"error": e.to_string()}),
        };
        to_jstring(&env, serde_json::to_string(&out).unwrap())
    }

    /// `nativeArscInfo(String path) -> String` (JSON).
    #[no_mangle]
    pub extern "system" fn Java_com_mtman_Jni_arscInfo<'a>(
        mut env: JNIEnv<'a>,
        _class: JClass<'a>,
        path: JString<'a>,
    ) -> JString<'a> {
        let path: String = env.get_string(&path).unwrap().into();
        let out = match fs::read_file(Path::new(&path)) {
            Ok(data) => match arsc::Arsc::parse(&data) {
                Ok(a) => {
                    let packages: Vec<_> = a
                        .packages
                        .iter()
                        .map(|p| {
                            json!({
                                "name": p.name,
                                "id": p.id,
                                "types": p.types.len(),
                                "entries": p.types.iter().map(|t| t.entry_count).sum::<usize>(),
                            })
                        })
                        .collect();
                    json!({
                        "global_strings": a.global_strings.len(),
                        "bad_strings": a.bad_strings,
                        "packages": packages,
                    })
                }
                Err(e) => json!({"error": format!("{e}")}),
            },
            Err(e) => json!({"error": e.to_string()}),
        };
        to_jstring(&env, serde_json::to_string(&out).unwrap())
    }

    /// `nativeGreeting() -> String` — JNI smoke test.
    #[no_mangle]
    pub extern "system" fn Java_com_mtman_Jni_greeting<'a>(
        env: JNIEnv<'a>,
        _class: JClass<'a>,
    ) -> JString<'a> {
        to_jstring(&env, "Hello from Rust!".to_string())
    }
}