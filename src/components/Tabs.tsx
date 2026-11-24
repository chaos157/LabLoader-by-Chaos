import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

export type TabDefinition = {
  id: string;
  label: string;
  count?: number;
};

export function Tabs({
  tabs,
  value,
  onChange,
}: {
  tabs: TabDefinition[];
  value: string;
  onChange: (id: string) => void;
}) {
  const container = useRef<HTMLDivElement>(null);
  const [indicator, setIndicator] = useState({ left: 0, width: 0 });
  const [ready, setReady] = useState(false);

  const measure = useCallback(() => {
    const node = container.current?.querySelector<HTMLButtonElement>(
      `[data-tab="${CSS.escape(value)}"]`,
    );
    if (!node || !container.current) return;
    const parent = container.current.getBoundingClientRect();
    const rect = node.getBoundingClientRect();
    setIndicator({ left: rect.left - parent.left, width: rect.width });
    setReady(true);
  }, [value]);

  useLayoutEffect(() => {
    measure();
  }, [measure]);

  useEffect(() => {
    const observer = new ResizeObserver(measure);
    if (container.current) observer.observe(container.current);
    return () => observer.disconnect();
  }, [measure]);

  return (
    <div className="tabs" ref={container} role="tablist">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          role="tab"
          aria-selected={value === tab.id}
          data-tab={tab.id}
          className={`tab ${value === tab.id ? "active" : ""}`}
          onClick={() => onChange(tab.id)}
        >
          {tab.label}
          {tab.count !== undefined && (
            <span className="tab-count">({tab.count})</span>
          )}
        </button>
      ))}
      <span
        className={`tab-indicator ${ready ? "ready" : ""}`}
        style={{
          transform: `translateX(${indicator.left}px)`,
          width: indicator.width,
        }}
      />
    </div>
  );
}
