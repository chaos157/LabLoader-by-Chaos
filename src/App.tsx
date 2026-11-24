import {
  HardDrive,
  Layers,
  LibraryBig,
  Loader2,
  PackageCheck,
  Play,
  Settings as SettingsIcon,
  ShieldOff,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { AddCustomModal } from "./components/AddCustomModal";
import { Select, Titlebar, Toaster } from "./components/common";
import { StoreProvider, useStore } from "./lib/store";
import { api } from "./lib/api";
import { InstalledPage } from "./pages/Installed";
import { LibraryPage } from "./pages/Library";
import { ModDetailPage } from "./pages/ModDetail";
import { ProfilesPage } from "./pages/Profiles";
import { SettingsPage } from "./pages/Settings";

type PageId = "library" | "profiles" | "installed" | "settings";

type DetailTarget = { namespace: string; name: string };

const navigation: { id: PageId; label: string; icon: typeof LibraryBig }[] = [
  { id: "library", label: "Library", icon: LibraryBig },
  { id: "profiles", label: "Profiles", icon: Layers },
  { id: "installed", label: "Installed", icon: PackageCheck },
  { id: "settings", label: "Settings", icon: SettingsIcon },
];

const pageMeta: Record<PageId, { title: string; subtitle: string }> = {
  library: {
    title: "Library",
    subtitle: "Every mod published for BONELAB on Thunderstore, ready to install.",
  },
  profiles: {
    title: "Profiles",
    subtitle: "Separate mod sets for every way you play. Share them as .blmp files.",
  },
  installed: {
    title: "Installed",
    subtitle: "Everything living in your MelonLoader Mods folder for this profile.",
  },
  settings: {
    title: "Settings",
    subtitle: "Game folder, profile storage, MelonLoader and appearance.",
  },
};

function MainShell() {
  const {
    activeProfile,
    profiles,
    settings,
    setActiveProfile,
    toast,
    ready,
    platform,
    storage,
    refreshStorage,
  } = useStore();
  const [page, setPage] = useState<PageId>("library");
  const [addOpen, setAddOpen] = useState(false);
  const [granting, setGranting] = useState(false);
  const [launching, setLaunching] = useState<"modded" | "vanilla" | "">("");
  const [detail, setDetail] = useState<DetailTarget | null>(null);
  const [detailClosing, setDetailClosing] = useState(false);
  const closeTimer = useRef<number | undefined>(undefined);

  function openDetail(target: DetailTarget) {
    if (closeTimer.current) window.clearTimeout(closeTimer.current);
    setDetailClosing(false);
    setDetail(target);
  }

  function closeDetail() {
    if (!detail || detailClosing) return;
    setDetailClosing(true);
    closeTimer.current = window.setTimeout(() => {
      setDetail(null);
      setDetailClosing(false);
    }, 220);
  }

  useEffect(() => {
    if (!detail) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeDetail();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [detail, detailClosing]);

  useEffect(
    () => () => {
      if (closeTimer.current) window.clearTimeout(closeTimer.current);
    },
    [],
  );

  async function switchProfile(profileId: string) {
    if (!profileId || profileId === activeProfile?.id) return;
    try {
      await setActiveProfile(profileId);
      toast("Active profile changed and deployed.", "info");
    } catch (error) {
      toast(String(error), "error");
    }
  }

  async function grantAccess() {
    setGranting(true);
    try {
      const granted = await api.requestStorageAccess();
      if (granted) {
        await refreshStorage();
        toast("Storage access granted. /storage/emulated/0/BLMP is ready.", "success");
      } else {
        toast(
          "In Android settings, enable “Allow access to manage all files” for LabLoader, then return here.",
          "info",
        );
      }
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setGranting(false);
    }
  }

  async function launch(vanilla: boolean) {
    setLaunching(vanilla ? "vanilla" : "modded");
    try {
      const result = await api.launchGame(activeProfile?.id, vanilla);
      toast(result.message, "success");
    } catch (error) {
      toast(String(error), "error");
      if (!settings?.gameDir) setPage("settings");
    } finally {
      setLaunching("");
    }
  }

  const meta =
    platform === "android"
      ? {
          ...pageMeta[page],
          installed: {
            ...pageMeta.installed,
            subtitle:
              "Everything deployed from this profile into your LemonLoader Mods folder.",
          },
          settings: {
            ...pageMeta.settings,
            subtitle:
              "Quest storage, loader folder, profile storage, LemonLoader and appearance.",
          },
        }
      : pageMeta[page];

  return (
    <div className="app">
      <Titlebar />
      <div className="shell">
        <aside className="sidebar">
          <nav className="nav">
            {navigation.map((item) => {
              const Icon = item.icon;
              return (
                <button
                  key={item.id}
                  className={`nav-item ${page === item.id ? "active" : ""}`}
                  onClick={() => setPage(item.id)}
                >
                  <Icon size={17} />
                  <span>{item.label}</span>
                  {item.id === "installed" && activeProfile && (
                    <span className="nav-count">{activeProfile.mods.length}</span>
                  )}
                </button>
              );
            })}
          </nav>

          <div className="sidebar-spacer" />

          <div className="quick-panel">
            <div className="quick-head">
              <span className="eyebrow">Active profile</span>
            </div>
            {profiles.length > 0 ? (
              <Select
                value={activeProfile?.id ?? ""}
                onChange={(value) => void switchProfile(value)}
                ariaLabel="Active profile"
              >
                {!activeProfile && <option value="">No profile</option>}
                {profiles.map((profile) => (
                  <option key={profile.id} value={profile.id}>
                    {profile.name}
                  </option>
                ))}
              </Select>
            ) : (
              <button
                className="btn btn-ghost btn-block"
                onClick={() => setPage("profiles")}
              >
                Create a profile
              </button>
            )}

            <button
              className="btn btn-primary btn-block"
              disabled={!ready || !!launching}
              onClick={() => void launch(false)}
              title="Deploy the active profile and start BONELAB with MelonLoader"
            >
              {launching === "modded" ? (
                <Loader2 className="spin" size={16} />
              ) : (
                <Play size={16} />
              )}
              Launch modded
            </button>
            <button
              className="btn btn-ghost btn-block"
              disabled={!ready || !!launching}
              onClick={() => void launch(true)}
              title="Start BONELAB without MelonLoader or mods"
            >
              {launching === "vanilla" ? (
                <Loader2 className="spin" size={16} />
              ) : (
                <ShieldOff size={16} />
              )}
              Launch vanilla
            </button>
          </div>

          <div className="sidebar-foot">
            <span className="dot" />
            <span>
              {settings?.gameDir ? "Game folder connected" : "Game folder not set"}
            </span>
          </div>
        </aside>

        <main className="content">
          <div className="page" key={page}>
            <header className="page-head">
              <div>
                <h1>{meta.title}</h1>
                <p>{meta.subtitle}</p>
              </div>
            </header>

            {storage?.mobile && !storage.granted && (
              <div className="notice storage-notice">
                <HardDrive size={18} />
                <div>
                  <strong>Quest storage access needed.</strong>
                  <span>
                    LabLoader needs All files access to create /storage/emulated/0/BLMP and
                    scan for MelonLoader/LemonLoader. Android/, Downloads/ and com.*
                    folders are never scanned.
                  </span>
                </div>
                <button
                  className="btn btn-secondary"
                  disabled={granting}
                  onClick={() => void grantAccess()}
                >
                  {granting ? <Loader2 className="spin" size={16} /> : "Grant access"}
                </button>
              </div>
            )}

            {storage?.mobile && storage.granted && !storage.loaderDir && (
              <div className="notice storage-notice">
                <HardDrive size={18} />
                <div>
                  <strong>No MelonLoader or LemonLoader folder found.</strong>
                  <span>
                    Create a top-level MelonLoader or LemonLoader folder in
                    /storage/emulated/0, then scan again in Settings. Folders starting
                    with “com.” are skipped.
                  </span>
                </div>
                <button className="btn btn-secondary" onClick={() => setPage("settings")}>
                  Open Settings
                </button>
              </div>
            )}

            {page === "library" && (
              <LibraryPage
                onAddCustom={() => setAddOpen(true)}
                onOpenMod={(namespace, name) => openDetail({ namespace, name })}
              />
            )}
            {page === "profiles" && (
              <ProfilesPage onAddCustom={() => setAddOpen(true)} />
            )}
            {page === "installed" && (
              <InstalledPage onAddCustom={() => setAddOpen(true)} />
            )}
            {page === "settings" && <SettingsPage />}
          </div>
        </main>
      </div>

      <AddCustomModal open={addOpen} onClose={() => setAddOpen(false)} />

      {detail && (
        <div className={`drawer-layer ${detailClosing ? "closing" : ""}`}>
          <div className="drawer-backdrop" onClick={closeDetail} />
          <aside
            className="mod-drawer"
            role="dialog"
            aria-modal="true"
            aria-label={`${detail.name} details`}
          >
            <header className="drawer-head">
              <div className="drawer-title">
                <span className="eyebrow">Thunderstore package</span>
                <strong>{detail.name}</strong>
              </div>
              <button
                className="icon-btn"
                aria-label="Close details"
                title="Close (Esc)"
                onClick={closeDetail}
              >
                <X size={17} />
              </button>
            </header>
            <ModDetailPage
              key={`${detail.namespace}/${detail.name}`}
              namespace={detail.namespace}
              name={detail.name}
              onOpenMod={(namespace, name) => openDetail({ namespace, name })}
            />
          </aside>
        </div>
      )}
    </div>
  );
}

export default function App() {
  return (
    <StoreProvider>
      <MainShell />
      <Toaster />
    </StoreProvider>
  );
}
