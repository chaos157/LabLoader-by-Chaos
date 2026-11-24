import {
  Download,
  Package,
  Plus,
  RefreshCw,
  Search,
  TriangleAlert,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { EmptyState, Select } from "../components/common";
import type { PackageSummary } from "../lib/api";
import { compact } from "../lib/format";
import { useStore } from "../lib/store";

const PAGE_SIZE = 48;

const sortLabels: Record<string, string> = {
  downloads: "Most downloaded",
  recent: "Recently updated",
  rating: "Best rated",
  name: "Name (A-Z)",
};

function ModCard({
  item,
  installed,
  onOpen,
}: {
  item: PackageSummary;
  installed: boolean;
  onOpen: (namespace: string, name: string) => void;
}) {
  return (
    <button
      className="mod-card"
      onClick={() => onOpen(item.namespace, item.name)}
      title={`Open ${item.name}`}
    >
      <div className="mod-card-top">
        <div className="mod-icon">
          {item.icon ? (
            <img src={item.icon} alt="" loading="lazy" />
          ) : (
            <Package size={26} />
          )}
        </div>
        <div className="mod-card-titles">
          <div className="mod-card-title">
            <span className="mod-name">{item.name}</span>
            {item.deprecated && (
              <span className="badge badge-warn">
                <TriangleAlert size={11} /> Deprecated
              </span>
            )}
          </div>
          <span className="mod-author">by {item.namespace}</span>
        </div>
      </div>

      <p className="mod-desc">{item.description || "No description provided."}</p>

      <div className="mod-card-foot">
        <span className="meta-item">
          <Download size={13} />
          {compact(item.downloads)}
        </span>
        <span className="version-pill">v{item.version}</span>
        {installed && <span className="badge badge-accent">Installed</span>}
      </div>
    </button>
  );
}

function SkeletonCard() {
  return (
    <div className="mod-card skeleton-card">
      <div className="mod-card-top">
        <div className="mod-icon skeleton" />
        <div className="mod-card-titles">
          <div className="skeleton skeleton-line" style={{ width: "60%" }} />
          <div className="skeleton skeleton-line" style={{ width: "35%" }} />
        </div>
      </div>
      <div className="skeleton skeleton-line" />
      <div className="skeleton skeleton-line" style={{ width: "80%" }} />
    </div>
  );
}

export function LibraryPage({
  onAddCustom,
  onOpenMod,
}: {
  onAddCustom: () => void;
  onOpenMod: (namespace: string, name: string) => void;
}) {
  const { packages, packagesLoading, packagesError, loadPackages, activeProfile } =
    useStore();
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState("downloads");
  const [visible, setVisible] = useState(PAGE_SIZE);
  const [refreshing, setRefreshing] = useState(false);

  useEffect(() => {
    if (packages.length === 0 && !packagesLoading && !packagesError) {
      void loadPackages();
    }
  }, [packages.length, packagesLoading, packagesError, loadPackages]);

  useEffect(() => {
    setVisible(PAGE_SIZE);
  }, [query, sort]);

  const installed = useMemo(() => {
    const set = new Set<string>();
    for (const entry of activeProfile?.mods ?? []) {
      if (entry.source === "thunderstore") {
        set.add(`${entry.namespace}/${entry.name}`.toLowerCase());
      }
    }
    return set;
  }, [activeProfile]);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const list = packages.filter((item) => {
      if (!needle) return true;
      return (
        item.name.toLowerCase().includes(needle) ||
        item.namespace.toLowerCase().includes(needle) ||
        item.description.toLowerCase().includes(needle)
      );
    });
    const sorted = [...list];
    if (sort === "recent") sorted.sort((a, b) => b.updated.localeCompare(a.updated));
    else if (sort === "rating") sorted.sort((a, b) => b.rating - a.rating);
    else if (sort === "name") sorted.sort((a, b) => a.name.localeCompare(b.name));
    else sorted.sort((a, b) => b.downloads - a.downloads);
    return sorted;
  }, [packages, query, sort]);

  async function refresh() {
    setRefreshing(true);
    try {
      await loadPackages(true);
    } finally {
      setRefreshing(false);
    }
  }

  return (
    <div className="stack">
      <div className="toolbar">
        <div className="search">
          <Search size={16} />
          <input
            placeholder="Search mods, authors or descriptions…"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          {query && (
            <button className="search-clear" onClick={() => setQuery("")}>
              Clear
            </button>
          )}
        </div>
        <Select value={sort} onChange={setSort} ariaLabel="Sort mods">
          {Object.entries(sortLabels).map(([value, label]) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
        </Select>
        <button
          className="btn btn-ghost"
          onClick={() => void refresh()}
          disabled={refreshing || packagesLoading}
          title="Refresh the Thunderstore index"
        >
          <RefreshCw size={16} className={refreshing || packagesLoading ? "spin" : ""} />
          Refresh
        </button>
        <button className="btn btn-primary" onClick={onAddCustom}>
          <Plus size={16} />
          Add custom mod
        </button>
      </div>

      <div className="toolbar-meta">
        <span>
          {packagesLoading
            ? "Loading BONELAB packages…"
            : `${filtered.length.toLocaleString()} mod${filtered.length === 1 ? "" : "s"}`}
        </span>
        {activeProfile && (
          <span className="muted">
            installing into <strong>{activeProfile.name}</strong>
          </span>
        )}
      </div>

      {packagesError && !packagesLoading && (
        <EmptyState
          icon={<TriangleAlert size={24} />}
          title="Could not load the library"
          body={packagesError}
          action={
            <button className="btn btn-primary" onClick={() => void loadPackages(true)}>
              <RefreshCw size={16} /> Try again
            </button>
          }
        />
      )}

      {packagesLoading && packages.length === 0 && (
        <div className="mod-grid">
          {Array.from({ length: 12 }).map((_, index) => (
            <SkeletonCard key={index} />
          ))}
        </div>
      )}

      {!packagesLoading && !packagesError && filtered.length === 0 && (
        <EmptyState
          icon={<Search size={24} />}
          title="No mods match that search"
          body="Try a different name, author or keyword."
          action={
            <button className="btn btn-ghost" onClick={() => setQuery("")}>
              Clear search
            </button>
          }
        />
      )}

      {filtered.length > 0 && (
        <>
          <div className="mod-grid">
            {filtered.slice(0, visible).map((item) => (
              <ModCard
                key={`${item.namespace}/${item.name}`}
                item={item}
                installed={installed.has(`${item.namespace}/${item.name}`.toLowerCase())}
                onOpen={onOpenMod}
              />
            ))}
          </div>

          {visible < filtered.length && (
            <div className="load-more">
              <button
                className="btn btn-ghost"
                onClick={() => setVisible((current) => current + PAGE_SIZE)}
              >
                Load more ({filtered.length - visible} left)
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
