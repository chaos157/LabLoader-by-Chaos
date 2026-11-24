import {
  Ban,
  Download,
  Loader2,
  Package,
  Play,
  Plus,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { EmptyState, Modal, Select, Spinner, Switch } from "../components/common";
import {
  api,
  type InstalledItem,
  type InstalledView,
  type LooseMod,
} from "../lib/api";
import { bytes } from "../lib/format";
import { useStore } from "../lib/store";

function ModRow({
  item,
  onToggle,
  onRemove,
  busy,
}: {
  item: InstalledItem;
  onToggle: (enabled: boolean) => void;
  onRemove: () => void;
  busy: boolean;
}) {
  return (
    <div className={`mod-row ${item.enabled ? "" : "is-disabled"}`}>
      <div className="mod-icon sm">
        {item.icon ? <img src={item.icon} alt="" /> : <Package size={20} />}
      </div>
      <div className="mod-row-main">
        <div className="mod-row-title">
          <strong>{item.name}</strong>
          {item.version && <span className="version-pill">v{item.version}</span>}
          <span className={`badge ${item.source === "custom" ? "badge-neutral" : "badge-soft"}`}>
            {item.source === "custom" ? "Custom" : "Thunderstore"}
          </span>
          {!item.enabled && <span className="badge badge-warn">Disabled</span>}
          {item.deployed && item.enabled && (
            <span className="badge badge-accent">In game</span>
          )}
        </div>
        <span className="muted small">
          {item.files.length} file{item.files.length === 1 ? "" : "s"}
          {item.namespace && item.namespace !== "Custom" ? ` · ${item.namespace}` : ""}
        </span>
      </div>
      <div className="mod-row-actions">
        {busy && <Spinner size={14} />}
        <Switch
          checked={item.enabled}
          onChange={onToggle}
          label={`Enable ${item.name}`}
        />
        <button className="icon-btn danger" title="Uninstall" onClick={onRemove} disabled={busy}>
          <Trash2 size={16} />
        </button>
      </div>
    </div>
  );
}

export function InstalledPage({ onAddCustom }: { onAddCustom: () => void }) {
  const {
    profiles,
    activeProfile,
    settings,
    setActiveProfile,
    refreshProfiles,
    toast,
    platform,
  } = useStore();
  const mobile = platform === "android";
  const [view, setView] = useState<InstalledView | null>(null);
  const [loading, setLoading] = useState(false);
  const [busyId, setBusyId] = useState("");
  const [removeTarget, setRemoveTarget] = useState<InstalledItem | null>(null);
  const [looseTarget, setLooseTarget] = useState<LooseMod | null>(null);
  const [redeploying, setRedeploying] = useState(false);
  const [launching, setLaunching] = useState(false);

  const profileId = activeProfile?.id ?? "";

  const load = useCallback(async () => {
    if (!profileId) {
      setView(null);
      return;
    }
    setLoading(true);
    try {
      setView(await api.listInstalled(profileId));
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setLoading(false);
    }
  }, [profileId, toast]);

  useEffect(() => {
    void load();
  }, [load]);

  async function toggle(item: InstalledItem, enabled: boolean) {
    setBusyId(item.id);
    setView((current) =>
      current
        ? {
            ...current,
            mods: current.mods.map((entry) =>
              entry.id === item.id ? { ...entry, enabled } : entry,
            ),
          }
        : current,
    );
    try {
      await api.setModEnabled(profileId, item.id, enabled);
      await refreshProfiles();
      await load();
    } catch (error) {
      toast(String(error), "error");
      await load();
    } finally {
      setBusyId("");
    }
  }

  async function uninstall() {
    if (!removeTarget) return;
    try {
      await api.uninstallMod(profileId, removeTarget.id);
      await refreshProfiles();
      await load();
      toast(`Removed ${removeTarget.name}.`, "success");
      setRemoveTarget(null);
    } catch (error) {
      toast(String(error), "error");
    }
  }

  async function handleLoose(mod: LooseMod, enabled: boolean) {
    try {
      await api.setLooseEnabled(mod.path, enabled);
      await load();
    } catch (error) {
      toast(String(error), "error");
    }
  }

  async function deleteLoose() {
    if (!looseTarget) return;
    try {
      await api.deleteLooseMod(looseTarget.path);
      await load();
      toast(`Deleted ${looseTarget.name} from Mods.`, "success");
      setLooseTarget(null);
    } catch (error) {
      toast(String(error), "error");
    }
  }

  async function redeploy() {
    if (!profileId) return;
    setRedeploying(true);
    try {
      const result = await api.redeployProfile(profileId);
      toast(`Deployed ${result.deployed} file(s) to BONELAB.`, "success");
      await load();
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setRedeploying(false);
    }
  }

  async function launch() {
    setLaunching(true);
    try {
      const result = await api.launchGame(profileId, false);
      toast(result.message, "success");
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setLaunching(false);
    }
  }

  async function changeProfile(nextId: string) {
    try {
      await setActiveProfile(nextId);
    } catch (error) {
      toast(String(error), "error");
    }
  }

  const enabledCount = view?.mods.filter((item) => item.enabled).length ?? 0;

  return (
    <div className="stack">
      <div className="toolbar">
        {profiles.length > 0 ? (
          <Select
            value={profileId}
            onChange={(value) => void changeProfile(value)}
            ariaLabel="Profile"
          >
            {profiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name}
              </option>
            ))}
          </Select>
        ) : (
          <span className="muted">No profile selected</span>
        )}
        <button className="btn btn-ghost" onClick={onAddCustom}>
          <Plus size={16} /> Add custom mod
        </button>
        <button
          className="btn btn-ghost"
          onClick={() => void redeploy()}
          disabled={!settings?.gameDir || redeploying || !profileId}
          title="Copy the active profile into the loader folder"
        >
          {redeploying ? (
            <Loader2 className="spin" size={16} />
          ) : (
            <RefreshCw size={16} />
          )}
          Deploy to game
        </button>
        <div className="toolbar-spacer" />
        <button
          className="btn btn-primary"
          onClick={() => void launch()}
          disabled={!profileId || launching}
        >
          {launching ? <Loader2 className="spin" size={16} /> : <Play size={16} />}
          Launch modded
        </button>
      </div>

      {!settings?.gameDir && (
        <div className="notice">
          <Ban size={18} />
          <div>
            <strong>{mobile ? "Loader folder not set." : "Game folder not set."}</strong>
            <span>
              {mobile
                ? "Scan for a MelonLoader or LemonLoader folder in Settings so LabLoader can deploy mods."
                : "Pick your BONELAB folder in Settings so LabLoader can deploy mods and install MelonLoader."}
            </span>
          </div>
        </div>
      )}

      <section className="panel">
        <header className="panel-head">
          <div>
            <h2>Mods in this profile</h2>
            <p className="muted small">
              {loading
                ? `Reading ${mobile ? "LemonLoader" : "MelonLoader"}/Mods…`
                : `${view?.mods.length ?? 0} installed · ${enabledCount} enabled`}
            </p>
          </div>
          {view?.gameDir && <span className="path-chip">{view.gameDir}</span>}
        </header>

        {!loading && (!view || view.mods.length === 0) ? (
          <EmptyState
            icon={<Download size={24} />}
            title="Nothing installed yet"
            body={`Head to the Library and pick a mod, or add a custom .dll. It will land in the ${mobile ? "LemonLoader" : "MelonLoader"} Mods folder.`}
          />
        ) : (
          <div className="mod-list">
            {view?.mods.map((item) => (
              <ModRow
                key={item.id}
                item={item}
                busy={busyId === item.id}
                onToggle={(enabled) => void toggle(item, enabled)}
                onRemove={() => setRemoveTarget(item)}
              />
            ))}
          </div>
        )}
      </section>

      <section className="panel">
        <header className="panel-head">
          <div>
            <h2>Loose files in Mods</h2>
            <p className="muted small">
              DLLs that were not installed by LabLoader, for example from a manual download.
              Deploying a profile disables these so only that profile's mods load.
            </p>
          </div>
        </header>
        {view && view.loose.length === 0 ? (
          <p className="muted panel-note">No untracked DLLs found.</p>
        ) : (
          <div className="mod-list">
            {view?.loose.map((mod) => (
              <div className={`mod-row ${mod.disabled ? "is-disabled" : ""}`} key={mod.path}>
                <div className="mod-icon sm">
                  <Package size={20} />
                </div>
                <div className="mod-row-main">
                  <div className="mod-row-title">
                    <strong>{mod.name}</strong>
                    <span className="badge badge-neutral">Manual</span>
                    {mod.disabled && <span className="badge badge-warn">Disabled</span>}
                  </div>
                  <span className="muted small">
                    {mod.path} · {bytes(mod.size)}
                  </span>
                </div>
                <div className="mod-row-actions">
                  <Switch
                    checked={!mod.disabled}
                    onChange={(enabled) => void handleLoose(mod, enabled)}
                    label={`Enable ${mod.name}`}
                  />
                  <button
                    className="icon-btn danger"
                    title="Delete file"
                    onClick={() => setLooseTarget(mod)}
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      <Modal
        open={!!removeTarget}
        title="Uninstall mod?"
        subtitle={removeTarget ? removeTarget.name : ""}
        onClose={() => setRemoveTarget(null)}
        footer={
          <>
            <button className="btn btn-ghost" onClick={() => setRemoveTarget(null)}>
              Cancel
            </button>
            <button className="btn btn-danger" onClick={() => void uninstall()}>
              <Trash2 size={16} /> Uninstall
            </button>
          </>
        }
      >
        <p className="modal-text">
          The mod files are removed from this profile and from the game's Mods folder.
        </p>
      </Modal>

      <Modal
        open={!!looseTarget}
        title="Delete file?"
        subtitle={looseTarget?.path}
        onClose={() => setLooseTarget(null)}
        footer={
          <>
            <button className="btn btn-ghost" onClick={() => setLooseTarget(null)}>
              Cancel
            </button>
            <button className="btn btn-danger" onClick={() => void deleteLoose()}>
              <Trash2 size={16} /> Delete
            </button>
          </>
        }
      >
        <p className="modal-text">
          This DLL is not tracked by any profile. Deleting it removes it from BONELAB
          permanently.
        </p>
      </Modal>
    </div>
  );
}
