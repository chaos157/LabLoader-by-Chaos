use serde::Serialize;
use std::path::{Path, PathBuf};

/// Shared external storage on Quest. BLMP is allowed to look at the top level
/// of this folder only, and never inside Android/, Downloads/ or com.* folders.
pub const STORAGE_ROOT: &str = "/storage/emulated/0";
pub const BLMP_DIR: &str = "/storage/emulated/0/BLMP";
pub const BONELAB_PACKAGE: &str = "com.StressLevelZero.BONELAB";

const LOADER_KEYS: [&str; 2] = ["melonloader", "lemonloader"];
const LOADER_FOLDERS: [&str; 4] = ["Mods", "Plugins", "UserLibs", "UserData"];

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatus {
    pub mobile: bool,
    pub granted: bool,
    pub root: String,
    pub blmp_dir: String,
    pub loader_dir: String,
    pub loaders: Vec<String>,
    pub message: String,
}

#[cfg(target_os = "android")]
fn android_context() -> Option<ndk_context::AndroidContext> {
    std::panic::catch_unwind(ndk_context::android_context).ok()
}

/// Android 11+ replaces the runtime storage prompt with the special
/// "All files access" permission. Quest 3 / 3S run Android 12, so this is the
/// check BLMP uses before touching /storage/emulated/0.
#[cfg(target_os = "android")]
pub fn is_storage_granted() -> bool {
    use jni::JavaVM;

    let Some(ctx) = android_context() else {
        return false;
    };
    let Ok(vm) = (unsafe { JavaVM::from_raw(ctx.vm().cast()) }) else {
        return false;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        return false;
    };
    let result = (|| -> jni::errors::Result<bool> {
        let environment = env.find_class("android/os/Environment")?;
        let granted =
            env.call_static_method(environment, "isExternalStorageManager", "()Z", &[])?;
        granted.z()
    })();
    result.unwrap_or(false)
}

#[cfg(not(target_os = "android"))]
pub fn is_storage_granted() -> bool {
    true
}

/// Creates /storage/emulated/0/BLMP and the folders BLMP keeps there.
#[cfg(target_os = "android")]
pub fn ensure_blmp_layout() -> Result<PathBuf, String> {
    let root = PathBuf::from(BLMP_DIR);
    for folder in ["Profiles", "Downloads", "Imports", "Exports"] {
        std::fs::create_dir_all(root.join(folder))
            .map_err(|e| format!("Could not create {}: {e}", root.join(folder).display()))?;
    }
    Ok(root)
}

#[cfg(not(target_os = "android"))]
pub fn ensure_blmp_layout() -> Result<PathBuf, String> {
    Err("The LabLoader storage folder is only created on Android.".into())
}

/// Top-level scan only: Android/, Downloads/ and com.* directories are skipped,
/// and nothing below the first level is ever inspected.
#[cfg(target_os = "android")]
pub fn scan_loaders() -> Vec<PathBuf> {
    let root = Path::new(STORAGE_ROOT);
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            if lower == "android" || lower == "downloads" || lower.starts_with("com.") {
                return None;
            }
            if LOADER_KEYS.iter().any(|key| lower.contains(key)) {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect();
    found.sort();
    found
}

#[cfg(not(target_os = "android"))]
pub fn scan_loaders() -> Vec<PathBuf> {
    Vec::new()
}

pub fn detect_loader() -> Option<PathBuf> {
    scan_loaders().into_iter().next()
}

/// MelonLoader/UserLibs/UserData naming, or LemonLoader's copy of the same
/// layout. Existing folders are left untouched.
#[cfg(target_os = "android")]
pub fn ensure_loader_layout(dir: &Path) -> Result<(), String> {
    if !dir.is_dir() {
        return Err(format!("Loader folder not found: {}", dir.display()));
    }
    for folder in LOADER_FOLDERS {
        std::fs::create_dir_all(dir.join(folder))
            .map_err(|e| format!("Could not create {}: {e}", dir.join(folder).display()))?;
    }
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn ensure_loader_layout(_dir: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "android")]
pub fn status(game_dir: &str) -> StorageStatus {
    let granted = is_storage_granted();
    let loaders = if granted {
        scan_loaders()
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let mut loader_dir = game_dir.to_string();
    if loader_dir.trim().is_empty() {
        if let Some(first) = loaders.first() {
            loader_dir = first.clone();
        }
    }

    let mut message = String::new();
    if !granted {
        message = "Allow LabLoader to manage all files so it can create /storage/emulated/0/BLMP and read MelonLoader/LemonLoader folders.".into();
    } else if let Err(error) = ensure_blmp_layout() {
        message = error;
    } else if loaders.is_empty() {
        message = format!(
            "No top-level MelonLoader or LemonLoader folder was found in {STORAGE_ROOT}. LabLoader does not scan Android/, Downloads/ or com.* folders."
        );
    } else if !loader_dir.is_empty() {
        let _ = ensure_loader_layout(Path::new(&loader_dir));
    }

    StorageStatus {
        mobile: true,
        granted,
        root: STORAGE_ROOT.into(),
        blmp_dir: BLMP_DIR.into(),
        loader_dir,
        loaders,
        message,
    }
}

#[cfg(not(target_os = "android"))]
pub fn status(game_dir: &str) -> StorageStatus {
    StorageStatus {
        mobile: false,
        granted: true,
        root: String::new(),
        blmp_dir: String::new(),
        loader_dir: game_dir.to_string(),
        loaders: Vec::new(),
        message: String::new(),
    }
}

/// Opens the system "All files access" page; Android does not show a normal
/// permission dialog for this permission.
#[cfg(target_os = "android")]
pub fn request_storage_access() -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    if is_storage_granted() {
        return Ok(());
    }

    let ctx = android_context().ok_or("Android context is not available yet.")?;
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|e| format!("Could not reach the Android runtime: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("Could not reach the Android runtime: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let result = (|| -> jni::errors::Result<()> {
        let action = env.new_string("android.settings.MANAGE_APP_ALL_FILES_ACCESS_PERMISSION")?;
        let package = env.new_string("package:com.blmp.manager")?;
        let uri = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&package)],
            )?
            .l()?;
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[JValue::Object(&action), JValue::Object(&uri)],
        )?;
        let _ = env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::Int(0x1000_0000)],
        )?;
        env.call_method(
            &context,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    })();

    if result.is_err() {
        // Some Quest builds do not expose the per-app page. Fall back to the
        // general All files access list where BLMP can be toggled manually.
        let _ = env.exception_clear();
        let fallback = (|| -> jni::errors::Result<()> {
            let action = env.new_string("android.settings.MANAGE_ALL_FILES_ACCESS_PERMISSION")?;
            let intent = env.new_object(
                "android/content/Intent",
                "(Ljava/lang/String;)V",
                &[JValue::Object(&action)],
            )?;
            let _ = env.call_method(
                &intent,
                "addFlags",
                "(I)Landroid/content/Intent;",
                &[JValue::Int(0x1000_0000)],
            )?;
            env.call_method(
                &context,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )?;
            Ok(())
        })();
        return fallback.map_err(|e| {
            format!("Could not open Android's All files access settings: {e}")
        });
    }

    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn request_storage_access() -> Result<(), String> {
    Ok(())
}

/// Starts BONELAB through its launch intent. LemonLoader hooks the game itself,
/// so BLMP only needs to deploy the profile before asking Android to launch it.
#[cfg(target_os = "android")]
pub fn launch_bonelab() -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = android_context().ok_or("Android context is not available yet.")?;
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|e| format!("Could not reach the Android runtime: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("Could not reach the Android runtime: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let result = (|| -> jni::errors::Result<()> {
        let manager = env
            .call_method(
                &context,
                "getPackageManager",
                "()Landroid/content/pm/PackageManager;",
                &[],
            )?
            .l()?;
        let package = env.new_string(BONELAB_PACKAGE)?;
        let intent = env
            .call_method(
                &manager,
                "getLaunchIntentForPackage",
                "(Ljava/lang/String;)Landroid/content/Intent;",
                &[JValue::Object(&package)],
            )?
            .l()?;
        if intent.is_null() {
            return Err(jni::errors::Error::NullPtr("BONELAB launch intent"));
        }
        env.call_method(
            &context,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    })();

    result.map_err(|_| {
        format!(
            "Could not start BONELAB. Make sure {BONELAB_PACKAGE} is installed on this headset."
        )
    })
}

#[cfg(not(target_os = "android"))]
pub fn launch_bonelab() -> Result<(), String> {
    Err("BONELAB launching through Android is only available on Quest.".into())
}

#[cfg(target_os = "android")]
pub fn open_url(url: &str) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = android_context().ok_or("Android context is not available yet.")?;
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|e| format!("Could not reach the Android runtime: {e}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("Could not reach the Android runtime: {e}"))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let result = (|| -> jni::errors::Result<()> {
        let action = env.new_string("android.intent.action.VIEW")?;
        let value = env.new_string(url)?;
        let uri = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&value)],
            )?
            .l()?;
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[JValue::Object(&action), JValue::Object(&uri)],
        )?;
        let _ = env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::Int(0x1000_0000)],
        )?;
        env.call_method(
            &context,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    })();

    result.map_err(|e| format!("Could not open the link: {e}"))
}

#[cfg(not(target_os = "android"))]
pub fn open_url(_url: &str) -> Result<(), String> {
    Err("Not available on this platform.".into())
}

/// Newest .blmp dropped into BLMP/Imports, used on Android instead of the
/// system picker (which hands back content:// URIs that std::fs cannot read).
#[cfg(target_os = "android")]
pub fn newest_import() -> Option<PathBuf> {
    let dir = PathBuf::from(BLMP_DIR).join("Imports");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .map(|ext| ext.to_string_lossy().eq_ignore_ascii_case("blmp"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort_by_key(|path| {
        path.metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|age| age.as_secs())
            .unwrap_or(0)
    });
    candidates.pop()
}

#[cfg(not(target_os = "android"))]
pub fn newest_import() -> Option<PathBuf> {
    None
}

#[cfg(target_os = "android")]
pub fn export_dir() -> PathBuf {
    PathBuf::from(BLMP_DIR).join("Exports")
}

#[cfg(not(target_os = "android"))]
pub fn export_dir() -> PathBuf {
    PathBuf::new()
}
