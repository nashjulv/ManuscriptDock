import { invoke, isTauri } from "@tauri-apps/api/core";
import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useI18n } from "./i18n";

export type TextSize = "small" | "default" | "large";
const STORAGE_KEY = "manuscriptdock.text-size.v1";
const SIZES: TextSize[] = ["small", "default", "large"];
function isTextSize(value: unknown): value is TextSize { return SIZES.includes(value as TextSize); }

interface TextSizeContextValue {
  size: TextSize;
  changeSize: (size: TextSize) => void;
  error: "load" | "save" | null;
  saving: boolean;
}
const TextSizeContext = createContext<TextSizeContextValue | null>(null);

export function TextSizeProvider({ children }: { children: ReactNode }) {
  const [size, setSize] = useState<TextSize>("default");
  const [error, setError] = useState<TextSizeContextValue["error"]>(null);
  const [saving, setSaving] = useState(false);
  const revision = useRef(0);
  const saveQueue = useRef(Promise.resolve());

  useEffect(() => {
    let active = true;
    const read = async () => {
      try {
        const value: unknown = isTauri()
          ? (await invoke<{ textSize: unknown }>("get_ui_preferences")).textSize
          : window.localStorage.getItem(STORAGE_KEY) ?? "default";
        if (!isTextSize(value)) throw new Error("UI_PREFERENCES_INVALID");
        // An older startup read must never undo a choice the user already made.
        if (active && revision.current === 0) setSize(value);
      } catch {
        if (active && revision.current === 0) setError("load");
      }
    };
    void read();
    return () => { active = false; };
  }, []);

  useLayoutEffect(() => {
    document.documentElement.dataset.textSize = size;
  }, [size]);

  const changeSize = useCallback((next: TextSize) => {
    setSize(next);
    setError(null);
    setSaving(true);
    const current = ++revision.current;
    // Preserve last-choice-wins ordering even when disk writes have different latencies.
    saveQueue.current = saveQueue.current.then(async () => {
      try {
        if (isTauri()) await invoke("save_ui_preferences", { textSize: next });
        else window.localStorage.setItem(STORAGE_KEY, next);
      } catch {
        if (revision.current === current) setError("save");
      } finally {
        if (revision.current === current) setSaving(false);
      }
    });
  }, []);

  const value = useMemo(() => ({ size, changeSize, error, saving }), [size, changeSize, error, saving]);
  return <TextSizeContext.Provider value={value}>{children}</TextSizeContext.Provider>;
}

export function TextSizeSettings() {
  const context = useContext(TextSizeContext);
  if (!context) throw new Error("TextSizeSettings requires TextSizeProvider");
  const { size, changeSize, error } = context;
  const { text } = useI18n();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLElement>(null);
  const labels = { small: text("更小", "Smaller"), default: text("默认", "Default"), large: text("更大", "Larger") };
  const errorMessage = error === "load"
    ? text("无法读取字体设置，已使用默认大小。点击任一档位重新保存。", "Text settings could not be read. Default size is active. Click any size to save again.")
    : text("字体大小已生效，但未能保存。点击任一档位重试。", "Text size is applied but could not be saved. Click any size to retry.");

  useEffect(() => {
    if (!open) return;
    panelRef.current?.querySelector<HTMLButtonElement>("button[aria-pressed=true]")?.focus();
    const outside = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    // Pointer clicks may blur a control with relatedTarget=null before its click fires.
    // Only an actual outside focus target should dismiss the popover.
    const focusOutside = (event: FocusEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("focusin", focusOutside);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("focusin", focusOutside);
    };
  }, [open]);

  const close = () => { setOpen(false); triggerRef.current?.focus(); };
  return <div className="text-size-settings" ref={rootRef}
    onKeyDown={(event) => { if (event.key === "Escape" && open) { event.preventDefault(); event.stopPropagation(); close(); } }}>
    <button ref={triggerRef} type="button" className="bar-button text-size-trigger" aria-label={text("字体大小", "Text size")}
      title={error ? errorMessage : text("字体大小", "Text size")} aria-haspopup="dialog" aria-expanded={open} aria-controls="text-size-popover"
      onClick={() => setOpen((previous) => !previous)}><span aria-hidden="true">Aa</span>{error ? <span className="text-size-error-dot" aria-hidden="true" /> : null}</button>
    {open ? <section ref={panelRef} id="text-size-popover" className="text-size-popover" role="dialog" aria-label={text("字体大小", "Text size")}>
      {SIZES.map((option) => <button key={option} className="text-size-option" type="button" aria-pressed={size === option}
        title={error ? errorMessage : undefined} onClick={() => changeSize(option)}>{labels[option]}</button>)}
    </section> : null}
    {error ? <span className="sr-only" role="alert">{errorMessage}</span> : null}
  </div>;
}
