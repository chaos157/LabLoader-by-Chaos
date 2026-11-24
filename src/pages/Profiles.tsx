import { open, save } from "@tauri-apps/plugin-dialog";
import { Download, Layers, Loader2, Play, Plus, Trash2, Upload } from "lucide-react";
import { useState } from "react";
import { api, type Profile } from "../lib/api";
import { fromUnix } from "../lib/format";
import { useStore } from "../lib/store";
import { EmptyState, Field, Modal, Spinner } from "../components/common";

function exportFileName(name: string): string {
  return `${name.replace(/[<>:"/\\|?*]/g, "-").trim() || "profile"}.blmp`;
}

export function ProfilesPage({ onAddCustom }: { onAddCustom: () => void }) {
  const {
    profiles,
    activeProfile,
    setActiveProfile,
    refreshProfiles,
    refreshSettings,
    toast,
    platform,
  } = useStore();
  const mobile = platform === "android";
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [creatingBusy, setCreatingBusy] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Profile | null>(null);
  const [busyId, setBusyId] = useState("");

  async function createProfile() {
    setCreatingBusy(true);
    try {
      const result = await api.createProfile(newName);
      await refreshProfiles();
      await refreshSettings();
      toast(result.message, "success");
      setCreating(false);
      setNewName("");
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setCreatingBusy(false);
    }
  }

  async function activate(profile: Profile) {
    setBusyId(profile.id);
    try {
      await setActiveProfile(profile.id);
      toast(`“${profile.name}” is now active and deployed.`, "success");
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setBusyId("");
    }
  }

  async function exportProfile(profile: Profile) {
    let destination = "";
    if (!mobile) {
      const picked = await save({
        defaultPath: exportFileName(profile.name),
        filters: [{ name: "BONELAB mod profile", extensions: ["blmp"] }],
      });
      if (!picked) return;
      destination = picked;
    }
    setBusyId(profile.id);
    try {
      const written = await api.exportProfile(profile.id, destination);
      toast(
        mobile
          ? `Exported to ${written}. It is in /storage/emulated/0/BLMP/Exports.`
          : `Exported to ${written}`,
        "success",
      );
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setBusyId("");
    }
  }

  async function importProfile() {
    let source = "";
    if (!mobile) {
      const picked = await open({
        multiple: false,
        filters: [{ name: "BONELAB mod profile", extensions: ["blmp"] }],
      });
      if (typeof picked !== "string") return;
      source = picked;
    } else {
      toast("Reading the newest .blmp from /storage/emulated/0/BLMP/Imports.", "info");
    }
    try {
      const profile = await api.importProfile(source);
      await refreshProfiles();
      toast(`Imported “${profile.name}”.`, "success");
    } catch (error) {
      toast(String(error), "error");
    }
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    try {
      await api.deleteProfile(deleteTarget.id);
      await refreshProfiles();
      toast(`Deleted “${deleteTarget.name}”.`, "success");
      setDeleteTarget(null);
    } catch (error) {
      toast(String(error), "error");
    }
  }

  async function launch(profile: Profile) {
    try {
      const result = await api.launchGame(profile.id, false);
      toast(result.message, "success");
    } catch (error) {
      toast(String(error), "error");
    }
  }

  return (
    <div className="stack">
      <div className="toolbar">
        <button className="btn btn-primary" onClick={() => setCreating(true)}>
          <Plus size={16} /> New profile
        </button>
        <button className="btn btn-ghost" onClick={() => void importProfile()}>
          <Upload size={16} /> Import .blmp
        </button>
        <button className="btn btn-ghost" onClick={onAddCustom}>
          <Plus size={16} /> Add custom mod
        </button>
        <div className="toolbar-spacer" />
        <span className="muted small">{profiles.length} profile(s)</span>
      </div>

      {profiles.length === 0 ? (
        <EmptyState
          icon={<Layers size={24} />}
          title="No profiles yet"
          body={
            mobile
              ? "Profiles keep separate mod sets under /storage/emulated/0/BLMP/Profiles. Scan your loader folder in Settings so they can be deployed."
              : "Profiles keep separate mod sets. Creating one installs the newest MelonLoader into BONELAB automatically."
          }
          action={
            <button className="btn btn-primary" onClick={() => setCreating(true)}>
              <Plus size={16} /> Create your first profile
            </button>
          }
        />
      ) : (
        <div className="profile-grid">
          {profiles.map((profile) => {
            const isActive = profile.id === activeProfile?.id;
            const enabled = profile.mods.filter((entry) => entry.enabled).length;
            return (
              <article className={`profile-card ${isActive ? "active" : ""}`} key={profile.id}>
                <div className="profile-card-head">
                  <div className="profile-avatar">{profile.name.slice(0, 1).toUpperCase()}</div>
                  <div>
                    <h3>{profile.name}</h3>
                    <span className="muted small">
                      Created {fromUnix(profile.createdAt)}
                    </span>
                  </div>
                  {isActive && <span className="badge badge-accent">Active</span>}
                </div>

                <div className="profile-stats">
                  <div>
                    <strong>{profile.mods.length}</strong>
                    <span>mods</span>
                  </div>
                  <div>
                    <strong>{enabled}</strong>
                    <span>enabled</span>
                  </div>
                  <div>
                    <strong>{profile.format.replace("blmp/", "v")}</strong>
                    <span>format</span>
                  </div>
                </div>

                <div className="profile-actions">
                  {isActive ? (
                    <button
                      className="btn btn-primary"
                      onClick={() => void launch(profile)}
                      disabled={busyId === profile.id}
                    >
                      <Play size={15} /> Launch
                    </button>
                  ) : (
                    <button
                      className="btn btn-secondary"
                      onClick={() => void activate(profile)}
                      disabled={busyId === profile.id}
                    >
                      {busyId === profile.id ? (
                        <Spinner size={14} />
                      ) : (
                        <Play size={15} />
                      )}
                      Use profile
                    </button>
                  )}
                  <button
                    className="icon-btn"
                    title="Export as .blmp"
                    onClick={() => void exportProfile(profile)}
                  >
                    <Download size={16} />
                  </button>
                  <button
                    className="icon-btn danger"
                    title="Delete profile"
                    onClick={() => setDeleteTarget(profile)}
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </article>
            );
          })}
        </div>
      )}

      <Modal
        open={creating}
        title="New profile"
        subtitle={
          mobile
            ? "Profiles are stored in /storage/emulated/0/BLMP/Profiles and deploy to your LemonLoader folder."
            : "The newest MelonLoader release is installed into BONELAB automatically."
        }
        onClose={() => setCreating(false)}
        footer={
          <>
            <button className="btn btn-ghost" onClick={() => setCreating(false)}>
              Cancel
            </button>
            <button
              className="btn btn-primary"
              onClick={() => void createProfile()}
              disabled={creatingBusy || !newName.trim()}
            >
              {creatingBusy ? <Loader2 className="spin" size={16} /> : <Plus size={16} />}
              Create profile
            </button>
          </>
        }
      >
        <Field label="Profile name" hint="For example: Modded, Vanilla+, Fusion, Testing.">
          <input
            className="input"
            autoFocus
            value={newName}
            placeholder="My loadout"
            onChange={(event) => setNewName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && newName.trim()) void createProfile();
            }}
          />
        </Field>
      </Modal>

      <Modal
        open={!!deleteTarget}
        title="Delete profile?"
        subtitle={deleteTarget ? `“${deleteTarget.name}” and its mod files will be removed.` : ""}
        onClose={() => setDeleteTarget(null)}
        footer={
          <>
            <button className="btn btn-ghost" onClick={() => setDeleteTarget(null)}>
              Keep it
            </button>
            <button className="btn btn-danger" onClick={() => void confirmDelete()}>
              <Trash2 size={16} /> Delete profile
            </button>
          </>
        }
      >
        <p className="modal-text">
          Export the profile first if you want to keep it. Deleting an active profile also
          removes its deployed mods from BONELAB.
        </p>
      </Modal>
    </div>
  );
}
