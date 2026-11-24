import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  api,
  type PackageSummary,
  type Profile,
  type Settings,
  type StorageStatus,
} from "./api";
import { applyTheme } from "./themes";

export type ToastKind = "success" | "error" | "info";

export type Toast = {
  id: number;
  kind: ToastKind;
  message: string;
};

type Store = {
  settings: Settings | null;
  profiles: Profile[];
  activeProfile: Profile | null;
  packages: PackageSummary[];
  packagesLoading: boolean;
  packagesError: string;
  toasts: Toast[];
  ready: boolean;
  platform: string;
  storage: StorageStatus | null;
  refreshSettings: () => Promise<void>;
  refreshStorage: () => Promise<StorageStatus | null>;
  refreshProfiles: () => Promise<void>;
  saveSettings: (patch: Partial<Settings>) => Promise<Settings>;
  loadPackages: (force?: boolean) => Promise<void>;
  setActiveProfile: (profileId: string) => Promise<void>;
  toast: (message: string, kind?: ToastKind) => void;
  dismissToast: (id: number) => void;
};

const StoreContext = createContext<Store | null>(null);

export function StoreProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [packages, setPackages] = useState<PackageSummary[]>([]);
  const [packagesLoading, setPackagesLoading] = useState(false);
  const [packagesError, setPackagesError] = useState("");
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [ready, setReady] = useState(false);
  const [platform, setPlatform] = useState("");
  const [storage, setStorage] = useState<StorageStatus | null>(null);
  const nextToastId = useRef(1);

  const dismissToast = useCallback((id: number) => {
    setToasts((current) => current.filter((entry) => entry.id !== id));
  }, []);

  const toast = useCallback(
    (message: string, kind: ToastKind = "info") => {
      const id = nextToastId.current++;
      setToasts((current) => [...current.slice(-3), { id, kind, message }]);
      window.setTimeout(() => dismissToast(id), kind === "error" ? 7000 : 4200);
    },
    [dismissToast],
  );

  const refreshSettings = useCallback(async () => {
    const next = await api.getSettings();
    setSettings(next);
    applyTheme(next.theme);
  }, []);

  const refreshProfiles = useCallback(async () => {
    const list = await api.listProfiles();
    setProfiles(list);
  }, []);

  const refreshStorage = useCallback(async () => {
    try {
      const next = await api.storageStatus();
      setStorage(next);
      if (next.mobile && next.granted) {
        const nextSettings = await api.getSettings();
        setSettings(nextSettings);
        applyTheme(nextSettings.theme);
        const list = await api.listProfiles();
        setProfiles(list);
      }
      return next;
    } catch {
      return null;
    }
  }, []);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const currentPlatform = await api.getPlatform().catch(() => "");
        if (alive) setPlatform(currentPlatform);
        const currentStorage = await api.storageStatus().catch(() => null);
        if (alive) setStorage(currentStorage);

        const next = await api.getSettings();
        if (!alive) return;
        setSettings(next);
        applyTheme(next.theme);
        const list = await api.listProfiles();
        if (alive) setProfiles(list);
      } catch (error) {
        if (alive) toast(String(error), "error");
      } finally {
        if (alive) setReady(true);
      }
    })();
    return () => {
      alive = false;
    };
  }, [toast]);

  useEffect(() => {
    const onVisible = () => {
      if (document.visibilityState === "visible" && platform === "android") {
        void refreshStorage();
      }
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  }, [platform, refreshStorage]);

  const saveSettings = useCallback(
    async (patch: Partial<Settings>) => {
      const current = settings ?? (await api.getSettings());
      const next = { ...current, ...patch };
      const saved = await api.saveSettings(next);
      setSettings(saved);
      if (saved.theme !== current.theme) applyTheme(saved.theme);
      return saved;
    },
    [settings],
  );

  const loadPackages = useCallback(
    async (force = false) => {
      setPackagesLoading(true);
      setPackagesError("");
      try {
        const list = await api.fetchPackages(force);
        setPackages(list);
      } catch (error) {
        setPackagesError(String(error));
      } finally {
        setPackagesLoading(false);
      }
    },
    [],
  );

  const setActiveProfile = useCallback(
    async (profileId: string) => {
      await api.setActiveProfile(profileId);
      const next = await api.getSettings();
      setSettings(next);
      await refreshProfiles();
    },
    [refreshProfiles],
  );

  const activeProfile = useMemo(
    () => profiles.find((profile) => profile.id === settings?.activeProfile) ?? null,
    [profiles, settings?.activeProfile],
  );

  const value = useMemo<Store>(
    () => ({
      settings,
      profiles,
      activeProfile,
      packages,
      packagesLoading,
      packagesError,
      toasts,
      ready,
      platform,
      storage,
      refreshSettings,
      refreshStorage,
      refreshProfiles,
      saveSettings,
      loadPackages,
      setActiveProfile,
      toast,
      dismissToast,
    }),
    [
      settings,
      profiles,
      activeProfile,
      packages,
      packagesLoading,
      packagesError,
      toasts,
      ready,
      platform,
      storage,
      refreshSettings,
      refreshStorage,
      refreshProfiles,
      saveSettings,
      loadPackages,
      setActiveProfile,
      toast,
      dismissToast,
    ],
  );

  return <StoreContext.Provider value={value}>{children}</StoreContext.Provider>;
}

export function useStore(): Store {
  const store = useContext(StoreContext);
  if (!store) throw new Error("useStore must be used inside StoreProvider");
  return store;
}
