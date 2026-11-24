import { open } from "@tauri-apps/plugin-dialog";
import {
  Check,
  FolderOpen,
  Gamepad2,
  HardDrive,
  Loader2,
  RefreshCw,
  Save,
  Sparkles,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Field } from "../components/common";
import { api, type MelonStatus } from "../lib/api";
import { useStore } from "../lib/store";
import { themes } from "../lib/themes";

export function SettingsPage() {
  const {
    settings,
    saveSettings,
    refreshProfiles,
    toast,
    platform,
    storage,
    refreshStorage,
  } = useStore();
  const mobile = platform === "android";
  const [gameDir, setGameDir] = useState(settings?.gameDir ?? "");
  const [profilesDir, setProfilesDir] = useState(settings?.profilesDir ?? "");
  const [saving, setSaving] = useState(false);
  const [granting, setGranting] = useState(false);
  const [detecting, setDetecting] = useState<"steam" | "oculus" | "mobile" | "">("");
  const [melon, setMelon] = useState<MelonStatus | null>(null);
  const [melonBusy, setMelonBusy] = useState(false);

  useEffect(() => {
    setGameDir(settings?.gameDir ?? "");
    setProfilesDir(settings?.profilesDir ?? "");
  }, [settings?.gameDir, settings?.profilesDir]);

  const loadMelon = useCallback(async () => {
    try {
      setMelon(await api.melonloaderStatus());
    } catch {
      setMelon(null);
    }
  }, []);

  useEffect(() => {
    void loadMelon();
  }, [loadMelon, settings?.gameDir]);

  async function saveLocations() {
    setSaving(true);
    try {
      await saveSettings({ gameDir, profilesDir });
      await refreshProfiles();
      await loadMelon();
      await refreshStorage();
      toast("Locations saved.", "success");
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setSaving(false);
    }
  }

  async function detect(launcher: "steam" | "oculus" | "mobile") {
    setDetecting(launcher);
    try {
      const result = await api.detectGame(launcher);
      if (!result.found) {
        toast(result.message, "error");
        return;
      }
      setGameDir(result.path);
      await saveSettings({ gameDir: result.path });
      await loadMelon();
      await refreshStorage();
      toast(result.message, "success");
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setDetecting("");
    }
  }

  async function grantStorage() {
    setGranting(true);
    try {
      const granted = await api.requestStorageAccess();
      if (granted) {
        await refreshStorage();
        toast("/storage/emulated/0/BLMP is ready.", "success");
      } else {
        toast(
          "Turn on “Allow access to manage all files” for LabLoader in Android settings, then tap Check again.",
          "info",
        );
      }
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setGranting(false);
    }
  }

  async function checkStorage() {
    const next = await refreshStorage();
    if (next?.granted) {
      if (next.loaderDir) setGameDir(next.loaderDir);
      await loadMelon();
      toast(
        next.loaderDir
          ? `Storage ready. Using ${next.loaderDir}.`
          : "Storage ready. No top-level MelonLoader or LemonLoader folder was found.",
        next.loaderDir ? "success" : "info",
      );
    } else {
      toast("Storage access is still off.", "error");
    }
  }

  async function browseGame() {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string") setGameDir(picked);
  }

  async function browseProfiles() {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string") setProfilesDir(picked);
  }

  async function updateMelon() {
    setMelonBusy(true);
    try {
      const status = await api.installMelonloader();
      setMelon(status);
      toast(`MelonLoader ${status.installed} installed.`, "success");
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setMelonBusy(false);
    }
  }

  async function pickTheme(themeId: string) {
    try {
      await saveSettings({ theme: themeId });
    } catch (error) {
      toast(String(error), "error");
    }
  }

  return (
    <div className="settings">
      {mobile && (
        <section className="panel">
          <header className="panel-head">
            <div>
              <h2>
                <HardDrive size={18} /> Quest storage
              </h2>
              <p className="muted small">
                LabLoader creates /storage/emulated/0/BLMP for profiles, imports and exports.
                It only scans the top level of /storage/emulated/0 for a MelonLoader or
                LemonLoader folder; Android, Downloads and com.* folders are never opened.
              </p>
            </div>
            <span className={`badge ${storage?.granted ? "badge-accent" : "badge-warn"}`}>
              {storage?.granted ? "Granted" : "Needs access"}
            </span>
          </header>

          <div className="info-grid">
            <div className="info-tile">
              <span className="eyebrow">Data folder</span>
              <strong>{storage?.blmpDir || "/storage/emulated/0/BLMP"}</strong>
            </div>
            <div className="info-tile">
              <span className="eyebrow">Loader folder</span>
              <strong>{storage?.loaderDir || "not found"}</strong>
            </div>
            <div className="info-tile">
              <span className="eyebrow">Created layout</span>
              <strong>Mods · Plugins · UserLibs · UserData</strong>
            </div>
          </div>

          {storage?.message && <p className="muted small">{storage.message}</p>}
          {storage && storage.loaders.length > 1 && (
            <p className="muted small">
              Also found: {storage.loaders.slice(1).join(", ")}
            </p>
          )}

          <div className="panel-actions">
            {!storage?.granted ? (
              <button
                className="btn btn-primary"
                onClick={() => void grantStorage()}
                disabled={granting}
              >
                {granting ? (
                  <Loader2 className="spin" size={16} />
                ) : (
                  <HardDrive size={16} />
                )}
                Open All files access
              </button>
            ) : (
              <button
                className="btn btn-secondary"
                onClick={() => void checkStorage()}
              >
                <RefreshCw size={16} /> Check again
              </button>
            )}
            <button
              className="btn btn-ghost"
              onClick={() => void detect("mobile")}
              disabled={!!detecting || !storage?.granted}
            >
              {detecting === "mobile" ? (
                <Loader2 className="spin" size={16} />
              ) : (
                <HardDrive size={16} />
              )}
              Scan for loader
            </button>
          </div>
        </section>
      )}

      <section className="panel">
        <header className="panel-head">
          <div>
            <h2>
              <Gamepad2 size={18} /> {mobile ? "Loader location" : "Game location"}
            </h2>
            <p className="muted small">
              {mobile
                ? "The MelonLoader or LemonLoader folder that contains Mods, Plugins, UserLibs and UserData."
                : "The folder that contains BONELAB_Steam_Windows64.exe or BONELAB_Oculus_Windows64.exe."}
            </p>
          </div>
          <span className={`badge ${gameDir ? "badge-accent" : "badge-warn"}`}>
            {gameDir ? "Connected" : "Not set"}
          </span>
        </header>

        <div className="setting-row">
          <Field label={mobile ? "MelonLoader / LemonLoader folder" : "BONELAB folder"}>
            <div className="input-row">
              <input
                className="input mono"
                value={gameDir}
                placeholder={
                  mobile
                    ? "/storage/emulated/0/LemonLoader"
                    : "C:\\Program Files (x86)\\Steam\\steamapps\\common\\BONELAB"
                }
                onChange={(event) => setGameDir(event.target.value)}
              />
              {!mobile && (
                <button className="btn btn-ghost" onClick={() => void browseGame()}>
                  <FolderOpen size={16} /> Browse
                </button>
              )}
            </div>
          </Field>
        </div>

        <div className="preset-row">
          <span className="eyebrow">{mobile ? "Scanner" : "Auto-detect"}</span>
          {mobile ? (
            <button
              className="btn btn-secondary"
              onClick={() => void detect("mobile")}
              disabled={!!detecting}
            >
              {detecting === "mobile" ? (
                <Loader2 className="spin" size={16} />
              ) : (
                <HardDrive size={16} />
              )}
              Scan Quest storage
            </button>
          ) : (
            <>
              <button
                className="btn btn-secondary"
                onClick={() => void detect("steam")}
                disabled={!!detecting}
              >
                {detecting === "steam" ? (
                  <Loader2 className="spin" size={16} />
                ) : (
                  <Gamepad2 size={16} />
                )}
                Steam
              </button>
              <button
                className="btn btn-secondary"
                onClick={() => void detect("oculus")}
                disabled={!!detecting}
              >
                {detecting === "oculus" ? (
                  <Loader2 className="spin" size={16} />
                ) : (
                  <Gamepad2 size={16} />
                )}
                Oculus / Meta Horizon
              </button>
            </>
          )}
        </div>
      </section>

      <section className="panel">
        <header className="panel-head">
          <div>
            <h2>
              <HardDrive size={18} /> Profiles folder
            </h2>
            <p className="muted small">
              {mobile
                ? "Profiles are stored inside /storage/emulated/0/BLMP/Profiles."
                : "Every profile is a folder with its own mod files and a profile.json."}
            </p>
          </div>
        </header>
        <div className="setting-row">
          <Field label="Store profiles in">
            <div className="input-row">
              <input
                className="input mono"
                value={profilesDir}
                onChange={(event) => setProfilesDir(event.target.value)}
              />
              {!mobile && (
                <button className="btn btn-ghost" onClick={() => void browseProfiles()}>
                  <FolderOpen size={16} /> Browse
                </button>
              )}
            </div>
          </Field>
        </div>
        <div className="panel-actions">
          <button
            className="btn btn-primary"
            onClick={() => void saveLocations()}
            disabled={saving}
          >
            {saving ? <Loader2 className="spin" size={16} /> : <Save size={16} />}
            Save locations
          </button>
        </div>
      </section>

      <section className="panel">
        <header className="panel-head">
          <div>
            <h2>
              <Sparkles size={18} /> {mobile ? "LemonLoader" : "MelonLoader"}
            </h2>
            <p className="muted small">
              {mobile
                ? "LemonLoader is installed and patched on the headset separately. LabLoader manages its mod folders."
                : "Installed into BONELAB automatically the first time you create a profile."}
            </p>
          </div>
          <span className="badge badge-soft">
            {melon?.installed ? melon.installed : "not installed"}
          </span>
        </header>

        <div className="info-grid">
          <div className="info-tile">
            <span className="eyebrow">Installed</span>
            <strong>{melon?.installed || "—"}</strong>
          </div>
          <div className="info-tile">
            <span className="eyebrow">{mobile ? "Manager" : "Newest release"}</span>
            <strong>{mobile ? "LemonLoader" : melon?.latest || "—"}</strong>
          </div>
          <div className="info-tile">
            <span className="eyebrow">Target</span>
            <strong>
              {melon?.gameReady
                ? mobile
                  ? "Loader folder"
                  : "BONELAB folder"
                : mobile
                  ? "No loader folder"
                  : "No game folder"}
            </strong>
          </div>
        </div>

        {melon?.message && <p className="muted small">{melon.message}</p>}

        {!mobile && (
          <div className="panel-actions">
            <button
              className="btn btn-secondary"
              onClick={() => void updateMelon()}
              disabled={melonBusy || !settings?.gameDir}
            >
              {melonBusy ? <Loader2 className="spin" size={16} /> : <RefreshCw size={16} />}
              Install / update MelonLoader
            </button>
          </div>
        )}
      </section>

      <section className="panel">
        <header className="panel-head">
          <div>
            <h2>
              <Sparkles size={18} /> Appearance
            </h2>
            <p className="muted small">
              Clean themes that apply instantly and are remembered between launches.
            </p>
          </div>
        </header>

        <div className="theme-grid">
          {themes.map((theme) => {
            const active = settings?.theme === theme.id;
            return (
              <button
                key={theme.id}
                className={`theme-card ${active ? "active" : ""}`}
                onClick={() => void pickTheme(theme.id)}
              >
                <span className="theme-swatch" style={{ background: theme.preview[0] }}>
                  <span
                    className="theme-swatch-panel"
                    style={{ background: theme.preview[1] }}
                  >
                    <span
                      className="theme-swatch-dot"
                      style={{ background: theme.preview[2] }}
                    />
                    <span
                      className="theme-swatch-line"
                      style={{ background: theme.preview[2] }}
                    />
                  </span>
                </span>
                <span className="theme-name">
                  {theme.name}
                  {active && <Check size={14} />}
                </span>
                <span className="theme-mode">{theme.mode}</span>
              </button>
            );
          })}
        </div>
      </section>
    </div>
  );
}
