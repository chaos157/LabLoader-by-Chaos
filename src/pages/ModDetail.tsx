import {
  BarChart3,
  BookOpen,
  Calendar,
  Download,
  ExternalLink,
  HardDrive,
  Heart,
  History,
  Info,
  Loader2,
  Package,
  RefreshCw,
  TriangleAlert,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { EmptyState, Select } from "../components/common";
import { Tabs } from "../components/Tabs";
import {
  api,
  parseDependency,
  type PackageDetail,
  type PackageVersion,
} from "../lib/api";
import { bytes, compact, relative, shortDate } from "../lib/format";
import { useStore } from "../lib/store";

function Markdown({ text, onLink }: { text: string; onLink: (url: string) => void }) {
  return (
    <div
      className="markdown"
      onClick={(event) => {
        const anchor = (event.target as HTMLElement).closest("a");
        if (!anchor) return;
        const href = anchor.getAttribute("href");
        if (href && /^https?:/i.test(href)) {
          event.preventDefault();
          onLink(href);
        }
      }}
    >
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{text}</ReactMarkdown>
    </div>
  );
}

function Facts({ detail, latest }: { detail: PackageDetail; latest?: PackageVersion }) {
  return (
    <div className="info-grid">
      <div className="info-tile">
        <span className="eyebrow">Downloads</span>
        <strong>{compact(detail.downloads)}</strong>
      </div>
      <div className="info-tile">
        <span className="eyebrow">Rating</span>
        <strong>{detail.rating}</strong>
      </div>
      <div className="info-tile">
        <span className="eyebrow">Versions</span>
        <strong>{detail.versions.length}</strong>
      </div>
      <div className="info-tile">
        <span className="eyebrow">Package size</span>
        <strong>{bytes(latest?.size ?? 0)}</strong>
      </div>
      <div className="info-tile">
        <span className="eyebrow">First uploaded</span>
        <strong>{shortDate(detail.firstUploaded)}</strong>
      </div>
      <div className="info-tile">
        <span className="eyebrow">Last updated</span>
        <strong>{shortDate(detail.updated)}</strong>
      </div>
    </div>
  );
}

export function ModDetailPage({
  namespace,
  name,
  onOpenMod,
}: {
  namespace: string;
  name: string;
  onOpenMod: (namespace: string, name: string) => void;
}) {
  const { profiles, activeProfile, settings, toast, refreshProfiles } = useStore();
  const [detail, setDetail] = useState<PackageDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [tab, setTab] = useState("details");
  const [profileId, setProfileId] = useState("");
  const [installing, setInstalling] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      setDetail(await api.fetchPackageDetail(namespace, name));
    } catch (caught) {
      setError(String(caught));
    } finally {
      setLoading(false);
    }
  }, [namespace, name]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (profileId) return;
    setProfileId(activeProfile?.id ?? settings?.activeProfile ?? "");
  }, [activeProfile, settings?.activeProfile, profileId]);

  const latest = detail?.versions[0];
  const deps = latest?.dependencies ?? [];

  const tabDefinitions = useMemo(
    () => [
      { id: "details", label: "Details" },
      { id: "required", label: "Required", count: deps.length },
      { id: "wiki", label: "Wiki" },
      { id: "changelog", label: "Changelog" },
      { id: "versions", label: "Versions", count: detail?.versions.length },
      { id: "analysis", label: "Analysis" },
    ],
    [deps.length, detail?.versions.length],
  );

  async function install(version: string) {
    if (!profileId) {
      toast("Create a profile first, then install.", "error");
      return;
    }
    setInstalling(version);
    try {
      const outcome = await api.installPackage(
        profileId,
        detail?.namespace ?? namespace,
        detail?.name ?? name,
        version,
      );
      toast(outcome.message, "success");
      await refreshProfiles();
    } catch (caught) {
      toast(String(caught), "error");
    } finally {
      setInstalling("");
    }
  }

  if (loading && !detail) {
    return (
      <div className="detail-scroll">
        <div className="detail-hero">
          <div className="detail-icon skeleton" />
          <div className="detail-info">
            <div className="skeleton skeleton-line" style={{ width: "35%", height: 28 }} />
            <div className="skeleton skeleton-line" style={{ width: "20%" }} />
            <div className="skeleton skeleton-line" style={{ width: "70%" }} />
          </div>
        </div>
        <div className="skeleton skeleton-block" />
      </div>
    );
  }

  if (error || !detail) {
    return (
      <div className="detail-scroll">
        <EmptyState
          icon={<TriangleAlert size={24} />}
          title="Package could not be loaded"
          body={error || "The package could not be found."}
          action={
            <button className="btn btn-primary" onClick={() => void load()}>
              <RefreshCw size={16} /> Retry
            </button>
          }
        />
      </div>
    );
  }

  return (
    <div className="detail-scroll">
      <div className="detail-hero">
        <div className="detail-icon">
          {detail.icon ? <img src={detail.icon} alt="" /> : <Package size={38} />}
        </div>
        <div className="detail-info">
          <div className="detail-title-row">
            <h1>{detail.name}</h1>
            {latest && <span className="version-pill big">v{latest.version}</span>}
            {detail.deprecated && (
              <span className="badge badge-warn">
                <TriangleAlert size={11} /> Deprecated
              </span>
            )}
          </div>
          <span className="detail-author">
            by{" "}
            <button
              className="link"
              onClick={() =>
                void api.openUrl(`https://thunderstore.io/c/bonelab/p/${detail.namespace}/`)
              }
            >
              {detail.namespace}
            </button>
          </span>
          <p className="detail-desc">{detail.description}</p>
          <div className="detail-meta">
            <span>
              <Download size={14} /> {compact(detail.downloads)} downloads
            </span>
            <span>
              <Heart size={14} /> {detail.rating} rating
            </span>
            <span>
              <Calendar size={14} /> updated {relative(detail.updated)}
            </span>
            <span>
              <HardDrive size={14} /> {bytes(latest?.size ?? 0)}
            </span>
          </div>
        </div>

        <div className="detail-actions">
          <Select
            value={profileId}
            onChange={setProfileId}
            ariaLabel="Install to profile"
            disabled={profiles.length === 0}
          >
            {profiles.length === 0 && <option value="">No profiles</option>}
            {profiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name}
              </option>
            ))}
          </Select>
          <button
            className="btn btn-primary"
            onClick={() => void install(latest?.version ?? "")}
            disabled={!latest || !!installing || profiles.length === 0}
          >
            {installing ? (
              <Loader2 className="spin" size={16} />
            ) : (
              <Download size={16} />
            )}
            {installing && installing !== latest?.version ? "Installing…" : "Install"}
          </button>
          <button
            className="btn btn-ghost"
            onClick={() => void api.openUrl(detail.packageUrl)}
          >
            <ExternalLink size={15} /> Thunderstore
          </button>
        </div>
      </div>

      <Tabs tabs={tabDefinitions} value={tab} onChange={setTab} />

      <div className="tab-panel" key={tab}>
        {tab === "details" && (
          <>
            {detail.readme.trim() ? (
              <Markdown text={detail.readme} onLink={(url) => void api.openUrl(url)} />
            ) : (
              <EmptyState
                icon={<BookOpen size={22} />}
                title="No description"
                body="The author did not publish a readme for this package."
              />
            )}
          </>
        )}

        {tab === "required" && (
          <div className="stack-sm">
            <p className="muted">
              Dependencies of version {latest?.version ?? "—"}. They are installed
              automatically alongside the mod.
            </p>
            {deps.length === 0 ? (
              <div className="dep-row">
                <Info size={16} />
                <span>This package has no dependencies.</span>
              </div>
            ) : (
              deps.map((dependency) => {
                const parsed = parseDependency(dependency);
                const isLoader =
                  parsed?.namespace.toLowerCase() === "lavagang" &&
                  parsed.name.toLowerCase() === "melonloader";
                return (
                  <div className="dep-row" key={dependency}>
                    <Package size={16} />
                    <div className="dep-main">
                      <strong>
                        {parsed ? `${parsed.namespace}/${parsed.name}` : dependency}
                      </strong>
                      <span className="muted small">
                        {isLoader
                          ? "MelonLoader is handled automatically by LabLoader"
                          : `required version ${parsed?.version ?? "—"}`}
                      </span>
                    </div>
                    {parsed && !isLoader && (
                      <button
                        className="btn btn-ghost btn-sm"
                        onClick={() => onOpenMod(parsed.namespace, parsed.name)}
                      >
                        Open
                      </button>
                    )}
                  </div>
                );
              })
            )}
          </div>
        )}

        {tab === "wiki" && (
          <div className="stack-sm">
            <Facts detail={detail} latest={latest} />
            <div className="link-list">
              <button className="link-row" onClick={() => void api.openUrl(detail.packageUrl)}>
                <ExternalLink size={16} />
                <span>
                  <strong>Thunderstore page</strong>
                  <span className="muted small">{detail.packageUrl}</span>
                </span>
              </button>
              {detail.websiteUrl && (
                <button
                  className="link-row"
                  onClick={() => void api.openUrl(detail.websiteUrl)}
                >
                  <ExternalLink size={16} />
                  <span>
                    <strong>Author website / repository</strong>
                    <span className="muted small">{detail.websiteUrl}</span>
                  </span>
                </button>
              )}
              {detail.donationLink && (
                <button
                  className="link-row"
                  onClick={() => void api.openUrl(detail.donationLink)}
                >
                  <Heart size={16} />
                  <span>
                    <strong>Support the author</strong>
                    <span className="muted small">{detail.donationLink}</span>
                  </span>
                </button>
              )}
              <button
                className="link-row"
                onClick={() =>
                  void api.openUrl(
                    `https://thunderstore.io/c/bonelab/p/${detail.namespace}/`,
                  )
                }
              >
                <Package size={16} />
                <span>
                  <strong>More from {detail.owner}</strong>
                  <span className="muted small">All packages by this author</span>
                </span>
              </button>
            </div>
          </div>
        )}

        {tab === "changelog" && (
          <>
            {detail.changelog.trim() ? (
              <Markdown text={detail.changelog} onLink={(url) => void api.openUrl(url)} />
            ) : (
              <EmptyState
                icon={<History size={22} />}
                title="No changelog"
                body="The author has not published changelog notes for this version."
              />
            )}
          </>
        )}

        {tab === "versions" && (
          <div className="version-list">
            {detail.versions.map((version) => (
              <div className="version-row" key={`${version.version}-${version.created}`}>
                <div className="version-main">
                  <strong>v{version.version}</strong>
                  <span className="muted small">
                    {shortDate(version.created)} · {compact(version.downloads)} downloads ·{" "}
                    {bytes(version.size)}
                  </span>
                  {version.description && (
                    <span className="version-desc">{version.description}</span>
                  )}
                </div>
                <button
                  className="btn btn-secondary btn-sm"
                  onClick={() => void install(version.version)}
                  disabled={!!installing}
                >
                  {installing === version.version ? (
                    <Loader2 className="spin" size={14} />
                  ) : (
                    <Download size={14} />
                  )}
                  Install
                </button>
              </div>
            ))}
          </div>
        )}

        {tab === "analysis" && (
          <div className="stack-sm">
            <Facts detail={detail} latest={latest} />
            <div className="panel">
              <header className="panel-head">
                <div>
                  <h2>
                    <BarChart3 size={18} /> Downloads by version
                  </h2>
                  <p className="muted small">Latest releases on Thunderstore.</p>
                </div>
              </header>
              <div className="bar-chart">
                {detail.versions.slice(0, 8).map((version) => {
                  const max = Math.max(
                    ...detail.versions.slice(0, 8).map((entry) => entry.downloads),
                    1,
                  );
                  return (
                    <div className="bar-row" key={version.version}>
                      <span className="bar-label">v{version.version}</span>
                      <span className="bar-track">
                        <span
                          className="bar-fill"
                          style={{ width: `${Math.max(4, (version.downloads / max) * 100)}%` }}
                        />
                      </span>
                      <span className="bar-value">{compact(version.downloads)}</span>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
