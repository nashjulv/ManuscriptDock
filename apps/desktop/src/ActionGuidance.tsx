import { createContext, useContext, useEffect, useId, useRef, useState, type ButtonHTMLAttributes, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { useI18n } from "./i18n";

export type GuideStage = "source" | "materials" | "check" | "revision" | "versions" | "journals" | "attestation" | "submission" | "knowledge";
export interface Prerequisite {
  message: string;
  target?: string;
  scope?: string;
  destination?: GuideStage;
  itemId?: string;
  actionLabel?: string;
  prepare?: () => void;
}
export type PrerequisiteCheck = Prerequisite | false | null | (() => Prerequisite | false | null);
const Navigation = createContext<((stage: GuideStage, itemId?: string, target?: string) => void) | null>(null);
const RevealAfterChange = createContext<((change: () => void, target?: string) => void) | null>(null);
const Drafts = createContext<Map<string, string> | null>(null);
const activeHighlights = new WeakMap<HTMLElement, () => void>();

export function highlightGuidanceTarget(target: HTMLElement) {
  activeHighlights.get(target)?.();
  // Restart the short animation when the same missing step is clicked again.
  target.classList.remove("prerequisite-highlight");
  void target.offsetWidth;
  target.classList.add("prerequisite-highlight");
  const clear = () => {
    if (activeHighlights.get(target) !== clear) return;
    clearTimeout(timer);
    target.classList.remove("prerequisite-highlight");
    target.removeEventListener("input", clear);
    target.removeEventListener("change", clear);
    activeHighlights.delete(target);
  };
  const timer = setTimeout(clear, 4000);
  activeHighlights.set(target, clear);
  target.addEventListener("input", clear, { once: true });
  target.addEventListener("change", clear, { once: true });
  return clear;
}

export function GuidanceProvider({ onNavigate, children }: { onNavigate: (stage: GuideStage, itemId?: string) => void; children: ReactNode }) {
  const drafts = useRef(new Map<string, string>());
  const frame = useRef(0);
  const cleanupPending = useRef<(() => void) | null>(null);
  const cleanupHighlight = useRef<(() => void) | null>(null);
  useEffect(() => () => { cancelAnimationFrame(frame.current); cleanupPending.current?.(); cleanupHighlight.current?.(); }, []);
  function reveal(change: () => void, selectors?: string) {
    cancelAnimationFrame(frame.current);
    cleanupPending.current?.();
    cleanupHighlight.current?.();
    change();
    let observer: MutationObserver | null = null;
    let deadline: ReturnType<typeof setTimeout>;
    const stopWaiting = () => { observer?.disconnect(); clearTimeout(deadline); };
    cleanupPending.current = stopWaiting;
    const focus = () => {
      if (document.querySelector('[data-guidance-loading="true"]')) return;
      const target = queryTarget(document, selectors);
      if (target) {
        stopWaiting();
        revealGuidanceTarget(target);
        cleanupHighlight.current = highlightGuidanceTarget(target);
      }
    };
    frame.current = requestAnimationFrame(() => {
      observer = new MutationObserver(focus);
      observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["data-guidance-loading"] });
      deadline = setTimeout(() => { stopWaiting(); const fallback = document.querySelector<HTMLElement>("#operation-pane"); if (fallback) revealGuidanceTarget(fallback); }, 10000);
      focus();
    });
  }
  return <Navigation.Provider value={(stage, itemId, target) => reveal(() => onNavigate(stage, itemId), target)}><RevealAfterChange.Provider value={reveal}><Drafts.Provider value={drafts.current}>{children}</Drafts.Provider></RevealAfterChange.Provider></Navigation.Provider>;
}

export function useGuidanceDraft(key: string) {
  const drafts = useContext(Drafts);
  const [value, setValue] = useState(() => drafts?.get(key) ?? "");
  return [value, (next: string) => { drafts?.set(key, next); setValue(next); }] as const;
}

function queryTarget(root: ParentNode | null | undefined, selectors: string | undefined) {
  // These are ordered fallbacks, not a document-order CSS selector list.
  for (const selector of selectors?.split(", ") ?? []) {
    const target = root?.querySelector<HTMLElement>(selector);
    if (target) return target;
  }
  return null;
}

/** Resolve against the clicked card, so repeated journal and file controls never focus another card. */
function findTarget(issue: Prerequisite, origin: HTMLElement) {
  const root = issue.scope ? origin.closest(issue.scope) : document;
  return queryTarget(root, issue.target);
}

export function revealGuidanceTarget(element: HTMLElement) {
  for (let parent = element.parentElement; parent; parent = parent.parentElement) {
    if (parent instanceof HTMLDetailsElement) parent.open = true;
  }
  if (element instanceof HTMLDetailsElement) element.open = true;
  if (!element.matches("button,input,textarea,select,a[href],[tabindex]")) element.tabIndex = -1;
  element.scrollIntoView?.({ block: "center", behavior: "auto" });
  element.focus({ preventScroll: true });
}

export function journalPrerequisite(message: string, actionLabel: string): Prerequisite {
  return { message, destination: "journals", actionLabel, target: '[data-primary-requirements] .official-source-access input:not(:checked), [data-primary-requirements] .official-source-access button, [data-primary-requirements] .manual-requirement-input input, [data-guide-selection="true"], .journal-result-group button.secondary-button, #generate-recommendations' };
}

/** Only business prerequisites remain clickable; in-flight operations still use native disabled. */
export function GuidedButton({ prerequisite, onClick, children, ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { prerequisite?: PrerequisiteCheck }) {
  const { text } = useI18n();
  const navigate = useContext(Navigation);
  const revealAfterChange = useContext(RevealAfterChange);
  const origin = useRef<HTMLButtonElement>(null);
  const dialog = useRef<HTMLDivElement>(null);
  const [notice, setNotice] = useState<{ issue: Prerequisite; target: HTMLElement | null; modal: boolean } | null>(null);
  const id = useId();
  const pendingFrame = useRef(0);
  const close = () => { setNotice(null); origin.current?.focus(); };

  useEffect(() => () => cancelAnimationFrame(pendingFrame.current), []);
  useEffect(() => {
    if (!notice?.target) return;
    const target = notice.target;
    const previousDescription = target.getAttribute("aria-describedby");
    const clearHighlight = highlightGuidanceTarget(target);
    target.setAttribute("aria-describedby", [previousDescription, id].filter(Boolean).join(" "));
    revealGuidanceTarget(target);
    const clear = () => setNotice(null);
    target.addEventListener("input", clear, { once: true });
    target.addEventListener("change", clear, { once: true });
    return () => {
      clearHighlight();
      if (previousDescription) target.setAttribute("aria-describedby", previousDescription);
      else target.removeAttribute("aria-describedby");
      target.removeEventListener("input", clear);
      target.removeEventListener("change", clear);
    };
  }, [notice, id]);
  useEffect(() => {
    if (notice?.modal) dialog.current?.querySelector<HTMLButtonElement>("button")?.focus();
  }, [notice]);

  function locate(issue: Prerequisite) {
    if (issue.prepare && !issue.scope && revealAfterChange) {
      setNotice(null);
      revealAfterChange(issue.prepare, issue.target);
      return;
    }
    issue.prepare?.();
    let attempts = 0;
    const check = () => {
      if (!origin.current?.isConnected) return;
      const target = findTarget(issue, origin.current);
      if (target) setNotice({ issue, target, modal: false });
      else if (++attempts < 12) pendingFrame.current = requestAnimationFrame(check);
      else setNotice({ issue, target: null, modal: false });
    };
    cancelAnimationFrame(pendingFrame.current);
    check();
  }

  function go() {
    // Re-read the latest condition: a dialog can remain open while state changes.
    const issue = typeof prerequisite === "function" ? prerequisite() : prerequisite;
    if (!issue) { close(); return; }
    if (issue.destination && navigate) {
      setNotice(null);
      // The workspace provider owns pending focus work, and cancels it when switching manuscripts.
      navigate(issue.destination, issue.itemId, issue.target);
    } else locate(issue);
  }

  const message = notice ? <p id={id} className="prerequisite-notice" role="status">{notice.issue.message}</p> : null;
  return <>
    <button {...props} ref={origin} type={props.type ?? "button"} onClick={event => {
      const issue = typeof prerequisite === "function" ? prerequisite() : prerequisite;
      if (!issue) { setNotice(null); onClick?.(event); return; }
      event.preventDefault();
      if (issue.destination) setNotice({ issue, target: null, modal: true });
      else locate(issue);
    }}>{children}</button>
    {notice?.modal ? createPortal(<div className="prerequisite-backdrop" onClick={event => { if (event.target === event.currentTarget) close(); }}>
      <div className="prerequisite-dialog" ref={dialog} role="dialog" aria-modal="true" aria-labelledby={`${id}-title`} aria-describedby={id} onKeyDown={event => {
        if (event.key === "Escape") { event.stopPropagation(); close(); }
        if (event.key === "Tab") {
          const buttons = dialog.current?.querySelectorAll<HTMLButtonElement>("button");
          if (!buttons?.length) return;
          if (event.shiftKey && document.activeElement === buttons[0]) { event.preventDefault(); buttons[buttons.length - 1].focus(); }
          else if (!event.shiftKey && document.activeElement === buttons[buttons.length - 1]) { event.preventDefault(); buttons[0].focus(); }
        }
      }}><h3 id={`${id}-title`}>{text("先完成这一步", "Complete this step first")}</h3>{message}<div>
        {(notice.issue.destination && navigate) || notice.issue.prepare ? <button className="primary-button" type="button" disabled={props.disabled} onClick={go}>{notice.issue.actionLabel ?? text("前往处理", "Go to the required step")}</button> : null}
        <button className="secondary-button" type="button" onClick={close}>{text("暂不处理", "Not now")}</button>
      </div></div>
    </div>, document.body) : notice?.target?.parentElement ? createPortal(message, notice.target.closest("label")?.parentElement ?? notice.target.parentElement) : message}
  </>;
}
