import { invoke } from "@tauri-apps/api/core";

export type Settings = {
  gameDir: string;
  profilesDir: string;
  theme: string;
  activeProfile: string;
  melonloaderVersion: string;
};

export type StorageStatus = {
  mobile: boolean;
  granted: boolean;
  root: string;
  blmpDir: string;
  loaderDir: string;
  loaders: string[];
  message: string;
};

export type ModEntry = {
  id: string;
  name: string;
  namespace: string;
  source: string;
  version: string;
  icon: string;
  enabled: boolean;
  files: string[];
  installedAt: number;
};

export type Profile = {
  format: string;
  id: string;
  name: string;
  createdAt: number;
  mods: ModEntry[];
};

export type PackageSummary = {
  namespace: string;
  name: string;
  fullName: string;
  description: string;
  icon: string;
  version: string;
  downloads: number;
  rating: number;
  updated: string;
  packageUrl: string;
  versionCount: number;
  deprecated: boolean;
  websiteUrl: string;
  size: number;
};

export type PackageVersion = {
  version: string;
  downloads: number;
  size: number;
  created: string;
  description: string;
  downloadUrl: string;
  dependencies: string[];
};

export type PackageDetail = {
  namespace: string;
  name: string;
  owner: string;
  description: string;
  icon: string;
  packageUrl: string;
  websiteUrl: string;
  donationLink: string;
  downloads: number;
  rating: number;
  firstUploaded: string;
  updated: string;
  deprecated: boolean;
  readme: string;
  changelog: string;
  versions: PackageVersion[];
};

export type InstalledItem = ModEntry & { deployed: boolean };

export type LooseMod = {
  name: string;
  path: string;
  size: number;
  disabled: boolean;
};

export type InstalledView = {
  mods: InstalledItem[];
  loose: LooseMod[];
  gameDir: string;
};

export type PresetResult = {
  path: string;
  found: boolean;
  message: string;
};

export type MelonStatus = {
  installed: string;
  latest: string;
  gameReady: boolean;
  message: string;
};

export type InstallOutcome = {
  installed: number;
  message: string;
  melonLoader: string;
};

export type CreateProfileResult = {
  profile: Profile;
  melonLoader: string;
  message: string;
};

export type LaunchResult = {
  pid: number;
  exe: string;
  vanilla: boolean;
  message: string;
};

export type DeployResult = {
  deployed: number;
  removed: number;
};

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  detectGame: (launcher: "steam" | "oculus" | "mobile") =>
    invoke<PresetResult>("detect_game", { launcher }),

  storageStatus: () => invoke<StorageStatus>("storage_status"),
  requestStorageAccess: () => invoke<boolean>("request_storage_access"),
  getPlatform: () => invoke<string>("get_platform"),

  fetchPackages: (force = false) => invoke<PackageSummary[]>("fetch_packages", { force }),
  fetchPackageDetail: (namespace: string, name: string) =>
    invoke<PackageDetail>("fetch_package_detail", { namespace, name }),

  listProfiles: () => invoke<Profile[]>("list_profiles"),
  createProfile: (name: string) => invoke<CreateProfileResult>("create_profile", { name }),
  deleteProfile: (profileId: string) => invoke<void>("delete_profile", { profileId }),
  setActiveProfile: (profileId: string) => invoke<Profile>("set_active_profile", { profileId }),
  redeployProfile: (profileId: string) =>
    invoke<DeployResult>("redeploy_profile", { profileId }),

  installPackage: (profileId: string, namespace: string, name: string, version: string) =>
    invoke<InstallOutcome>("install_package", { profileId, namespace, name, version }),
  installCustomMod: (profileId: string, name: string, source: string) =>
    invoke<InstallOutcome>("install_custom_mod", { profileId, name, source }),
  uninstallMod: (profileId: string, modId: string) =>
    invoke<void>("uninstall_mod", { profileId, modId }),
  setModEnabled: (profileId: string, modId: string, enabled: boolean) =>
    invoke<void>("set_mod_enabled", { profileId, modId, enabled }),
  listInstalled: (profileId: string) =>
    invoke<InstalledView>("list_installed", { profileId }),
  deleteLooseMod: (relativePath: string) =>
    invoke<void>("delete_loose_mod", { relativePath }),
  setLooseEnabled: (relativePath: string, enabled: boolean) =>
    invoke<void>("set_loose_enabled", { relativePath, enabled }),

  exportProfile: (profileId: string, destination: string) =>
    invoke<string>("export_profile", { profileId, destination }),
  importProfile: (source: string) => invoke<Profile>("import_profile", { source }),

  melonloaderStatus: () => invoke<MelonStatus>("melonloader_status"),
  installMelonloader: () => invoke<MelonStatus>("install_melonloader"),

  launchGame: (profileId: string | undefined, vanilla: boolean) =>
    invoke<LaunchResult>("launch_game", { profileId: profileId ?? null, vanilla }),

  openUrl: (url: string) => invoke<void>("open_url", { url }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
};

export function parseDependency(dependency: string): {
  namespace: string;
  name: string;
  version: string;
} | null {
  const parts = dependency.split("-");
  if (parts.length < 3) return null;
  const [namespace, name, ...rest] = parts;
  return { namespace, name, version: rest.join("-") };
}
