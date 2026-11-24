import { open } from "@tauri-apps/plugin-dialog";
import { FolderSearch, Loader2, Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { useStore } from "../lib/store";
import { Field, Modal, Select } from "./common";

export function AddCustomModal({ open: isOpen, onClose }: { open: boolean; onClose: () => void }) {
  const { profiles, activeProfile, toast, refreshProfiles, platform } = useStore();
  const mobile = platform === "android";
  const [profileId, setProfileId] = useState("");
  const [name, setName] = useState("");
  const [source, setSource] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    setProfileId(activeProfile?.id ?? profiles[0]?.id ?? "");
    setName("");
    setSource("");
    setBusy(false);
  }, [isOpen, activeProfile?.id, profiles]);

  async function browse() {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "MelonLoader mod", extensions: ["dll", "zip"] }],
    });
    if (typeof picked === "string") setSource(picked);
  }

  async function submit() {
    if (!profileId) {
      toast("Create a profile first.", "error");
      return;
    }
    if (!source.trim()) {
      toast("Choose a .dll or .zip file, or paste a link.", "error");
      return;
    }
    setBusy(true);
    try {
      const outcome = await api.installCustomMod(profileId, name.trim(), source.trim());
      toast(outcome.message, "success");
      await refreshProfiles();
      onClose();
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal
      open={isOpen}
      title="Add a custom mod"
      subtitle={
        mobile
          ? "Add a mod from a link or from /storage/emulated/0/BLMP/Downloads."
          : "Drop in your own MelonLoader mod without going through Thunderstore."
      }
      onClose={onClose}
      footer={
        <>
          <button className="btn btn-ghost" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={() => void submit()} disabled={busy}>
            {busy ? <Loader2 className="spin" size={16} /> : <Plus size={16} />}
            Add mod
          </button>
        </>
      }
    >
      <div className="form-grid">
        <Field label="Profile">
          <Select value={profileId} onChange={setProfileId} ariaLabel="Target profile">
            {profiles.length === 0 && <option value="">No profiles yet</option>}
            {profiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name}
              </option>
            ))}
          </Select>
        </Field>

        <Field label="Display name" hint="Shown on the Installed page.">
          <input
            className="input"
            placeholder="My custom mod"
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </Field>

        <Field
          label="Mod file or link"
          hint={
            mobile
              ? "Use /storage/emulated/0/BLMP/Downloads/MyMod.dll or paste a direct link. A .zip is unpacked like a package."
              : "A .dll goes into Mods/, a .zip is unpacked like a package."
          }
        >
          <div className="input-row">
            <input
              className="input"
              placeholder={
                mobile
                  ? "/storage/emulated/0/BLMP/Downloads/MyMod.dll or https://…"
                  : "C:\\mods\\MyMod.dll or https://…"
              }
              value={source}
              onChange={(event) => setSource(event.target.value)}
            />
            {!mobile && (
              <button className="btn btn-ghost" onClick={() => void browse()} type="button">
                <FolderSearch size={16} />
                Browse
              </button>
            )}
          </div>
        </Field>
      </div>
    </Modal>
  );
}
