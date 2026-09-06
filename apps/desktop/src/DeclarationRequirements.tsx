import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { localizeBackendText, useI18n } from "./i18n";
import type { JournalRequirementItem, SubmissionMaterialCatalog, SubmissionMaterialChecklistItem } from "./App";

export type SubmissionStage = "initial" | "revision" | "accepted" | "unknown";
export type DeclarationDelivery = "manuscript" | "attachment" | "submission_system" | "unknown";
export interface DeclarationRequirement {
  delivery: DeclarationDelivery[]; stage: SubmissionStage; condition: string | null;
  applicability: "applicable" | "needs_review" | "not_applicable"; fileCount: number;
  allowedExtensions: string[]; signatureRequired: boolean; stampRequired: boolean;
  sharedFileAllowed: boolean; templateUrl: string | null; authorNote: string | null;
}
export interface DeclarationPlan {
  manuscriptVersion: number; targetSelectionId: string; requirementSnapshotId: string;
  stage: SubmissionStage; requirements: JournalRequirementItem[];
}
export interface DeclarationPlanUpdate {
  requirement?: JournalRequirementItem; stage?: SubmissionStage; materialId?: string; checklistItemId?: string;
}

const EMPTY_DECLARATION: DeclarationRequirement = { delivery: ["unknown"], stage: "unknown", condition: null, applicability: "needs_review", fileCount: 1, allowedExtensions: [], signatureRequired: false, stampRequired: false, sharedFileAllowed: false, templateUrl: null, authorNote: null };

export function DeclarationDetails({ item }: { item: SubmissionMaterialChecklistItem }) {
  const { text, locale } = useI18n();
  const declaration = item.declaration;
  if (!declaration) return null;
  const attachment = item.declarationAction === "attachment";
  const verifySignatures = attachment || item.declarationAction === "attestation";
  return <div className="declaration-details">
    <strong>{localizeBackendText(locale, `DECLARATION_${item.declarationAction?.toUpperCase()}`)}</strong>
    <small>{text("提交阶段：", "Stage: ")}{localizeBackendText(locale, `DECLARATION_STAGE_${declaration.stage.toUpperCase()}`)}</small>
    {declaration.condition ? <small>{text("适用条件：", "Applies when: ")}{declaration.condition}</small> : null}
    {verifySignatures && declaration.signatureRequired ? <small>{text("需作者核验签字完整性", "Author must verify all required signatures")}</small> : null}
    {verifySignatures && declaration.stampRequired ? <small>{text("需作者核验单位盖章", "Author must verify the institutional stamp")}</small> : null}
    {attachment && declaration.allowedExtensions.length ? <small>{text("官方限定格式：", "Official formats: ")}{declaration.allowedExtensions.join(", ").toUpperCase()}</small> : null}
    {attachment && declaration.templateUrl && /^https?:\/\//.test(declaration.templateUrl) ? <a href={declaration.templateUrl} target="_blank" rel="noreferrer">{text("查看官方模板", "View official template")}</a> : null}
  </div>;
}

export function DeclarationRequirements({ catalog, disabled, onUpdated }: { catalog: SubmissionMaterialCatalog; disabled: boolean; onUpdated: (catalog: SubmissionMaterialCatalog) => void }) {
  const { text, locale } = useI18n();
  const [editing, setEditing] = useState<JournalRequirementItem | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reuse, setReuse] = useState<Record<string, string>>({});
  const [reuseSlots, setReuseSlots] = useState<Record<string, string>>({});
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  const plan = catalog.declarationPlan;
  if (!plan) return null;
  async function save(update: DeclarationPlanUpdate) {
    if (busy || disabled) return;
    setBusy(true); setError(null);
    try {
      const result = await invoke<SubmissionMaterialCatalog>("update_declaration_plan", { workspaceId: catalog.workspaceId, update: { ...update, manuscriptVersion: plan!.manuscriptVersion, targetSelectionId: plan!.targetSelectionId, requirementSnapshotId: plan!.requirementSnapshotId } });
      if (mounted.current) { onUpdated(result); setEditing(null); }
    } catch (cause) { if (mounted.current) setError(String(cause)); }
    finally { if (mounted.current) setBusy(false); }
  }
  const patch = (change: Partial<JournalRequirementItem>) => setEditing(current => current ? { ...current, ...change } : null);
  const patchDeclaration = (change: Partial<DeclarationRequirement>) => setEditing(current => current ? { ...current, declaration: { ...(current.declaration ?? EMPTY_DECLARATION), ...change } } : null);
  const locked = disabled || busy;
  return <section className="declaration-requirements" aria-label={text("声明要求管理", "Declaration requirements")}>
    <header><h3>{text("声明与支持文件要求", "Declarations and supporting documents")}</h3>
      <label>{text("当前投稿阶段", "Current submission stage")}<select disabled={locked} value={plan.stage} onChange={event => void save({ stage: event.target.value as SubmissionStage })}>
        <option value="initial">{text("初次投稿", "Initial submission")}</option><option value="revision">{text("返修", "Revision")}</option><option value="accepted">{text("录用后", "After acceptance")}</option>
      </select></label>
    </header>
    <p>{text("按官方原文核验提交方式、条件和阶段。附件、正文声明和系统填写分别检查；后续阶段的材料不会阻断当前投稿。", "Verify delivery, conditions, and stage against the official source. Attachments, manuscript statements, and system entries are checked separately; later-stage documents do not block the current submission.")}</p>
    <div className="declaration-summary-list">{plan.requirements.map(requirement => {
      const attachments = catalog.checklist.filter(item => item.declarationRequirementId === requirement.id && item.declarationAction === "attachment");
      const attachment = attachments.find(item => item.id === reuseSlots[requirement.id]) ?? attachments[0];
      return <article key={requirement.id}><strong>{locale === "en" ? requirement.labelEn : requirement.label}</strong><blockquote>{requirement.evidenceExcerpt}</blockquote>
        {requirement.declaration?.authorNote ? <p>{text("作者核验依据：", "Author verification note: ")}{requirement.declaration.authorNote}</p> : null}
        <button type="button" className="text-button" disabled={locked} onClick={() => { setEditing(structuredClone(requirement)); setError(null); }}>{text("核验或调整要求", "Verify or adjust requirement")}</button>
        {attachment && attachment.status !== "not_applicable" && catalog.materials.length > 0 ? <div className="declaration-reuse">{attachments.length > 1 ? <label>{text("关联目标文件项", "Destination file slot")}<select disabled={locked} value={attachment.id} onChange={event => setReuseSlots(current => ({ ...current, [requirement.id]: event.target.value }))}>{attachments.map(item => <option key={item.id} value={item.id}>{locale === "en" ? item.labelEn : item.label}</option>)}</select></label> : null}<label>{text("关联已有文件", "Link a stored file")}<select disabled={locked} value={reuse[requirement.id] ?? ""} onChange={event => setReuse(current => ({ ...current, [requirement.id]: event.target.value }))}>
          <option value="">{text("选择文件（含历史材料）", "Choose a file (including history)")}</option>{catalog.materials.filter(material => material.kind === "declaration" || material.kind === "other").map(material => <option key={material.materialId} value={material.materialId}>{material.originalName} · v{material.manuscriptVersion}</option>)}
        </select></label><button className="secondary-button" type="button" disabled={locked || !reuse[requirement.id]} onClick={() => void save({ materialId: reuse[requirement.id], checklistItemId: attachment.id })}>{text("关联到此要求", "Link to this requirement")}</button></div> : null}
      </article>;
    })}</div>
    <button className="secondary-button" type="button" disabled={locked} onClick={() => setEditing({ id: "", category: "other_supporting_files", label: "", labelEn: "", obligation: "required", detail: "", sourceUrl: "", evidenceExcerpt: "", declaration: { ...EMPTY_DECLARATION } })}>{text("补充遗漏的声明要求", "Add a missing declaration requirement")}</button>
    {editing ? <form className="declaration-editor" aria-label={text("编辑声明要求", "Edit declaration requirement")} onSubmit={event => { event.preventDefault(); void save({ requirement: editing }); }}>
      <h4>{text("依据官方原文核验", "Verify against official text")}</h4>
      <label>{text("中文名称", "Chinese label")}<input required maxLength={100} value={editing.label} onChange={event => patch({ label: event.target.value })} /></label>
      <label>{text("英文名称", "English label")}<input required maxLength={200} value={editing.labelEn} onChange={event => patch({ labelEn: event.target.value })} /></label>
      <label>{text("官方来源地址", "Official source URL")}<input type="url" required readOnly={Boolean(editing.id && !editing.id.startsWith("requirement-manual-"))} value={editing.sourceUrl} onChange={event => patch({ sourceUrl: event.target.value })} /></label>
      <label>{text("官方原文", "Official source excerpt")}<textarea required minLength={12} readOnly={Boolean(editing.id && !editing.id.startsWith("requirement-manual-"))} value={editing.evidenceExcerpt} onChange={event => patch({ evidenceExcerpt: event.target.value })} /></label>
      <fieldset><legend>{text("提交方式（可多选）", "Delivery (select all that apply)")}</legend>{(["manuscript", "attachment", "submission_system", "unknown"] as DeclarationDelivery[]).map(delivery => <label key={delivery}><input type="checkbox" checked={editing.declaration?.delivery.includes(delivery) ?? false} onChange={event => {
        const current = editing.declaration?.delivery ?? [];
        patchDeclaration({ delivery: event.target.checked ? delivery === "unknown" ? [delivery] : [...current.filter(value => value !== "unknown"), delivery] : current.filter(value => value !== delivery) });
      }} />{localizeBackendText(locale, `DECLARATION_DELIVERY_${delivery.toUpperCase()}`)}</label>)}</fieldset>
      <label>{text("义务强度", "Obligation")}<select value={editing.obligation} onChange={event => patch({ obligation: event.target.value as JournalRequirementItem["obligation"] })}><option value="required">{text("必需", "Required")}</option><option value="recommended">{text("建议", "Recommended")}</option><option value="verify">{text("待核验", "Needs verification")}</option></select></label>
      <label>{text("要求的提交阶段", "Required submission stage")}<select value={editing.declaration?.stage} onChange={event => patchDeclaration({ stage: event.target.value as SubmissionStage })}>{(["initial", "revision", "accepted", "unknown"] as SubmissionStage[]).map(stage => <option key={stage} value={stage}>{localizeBackendText(locale, `DECLARATION_STAGE_${stage.toUpperCase()}`)}</option>)}</select></label>
      <label>{text("适用条件原文", "Applicability condition")}<textarea value={editing.declaration?.condition ?? ""} onChange={event => patchDeclaration({ condition: event.target.value || null })} /></label>
      <label>{text("对当前论文是否适用", "Applies to this manuscript")}<select value={editing.declaration?.applicability} onChange={event => patchDeclaration({ applicability: event.target.value as DeclarationRequirement["applicability"] })}><option value="needs_review">{text("待核验", "Needs verification")}</option><option value="applicable">{text("适用", "Applicable")}</option><option value="not_applicable">{text("不适用（需说明依据）", "Not applicable (explain why)")}</option></select></label>
      <label>{text("独立文件数量", "Independent file count")}<input type="number" min={1} max={20} required value={editing.declaration?.fileCount} onChange={event => patchDeclaration({ fileCount: Number(event.target.value) })} /></label>
      <label>{text("官方限定格式（无明确限制可留空）", "Official formats (leave blank if unspecified)")}<input placeholder="pdf, docx, png" value={editing.declaration?.allowedExtensions.join(", ") ?? ""} onChange={event => patchDeclaration({ allowedExtensions: event.target.value.toLowerCase().split(/[,，\s]+/).filter(Boolean) })} /></label>
      {([ ["signatureRequired", text("要求签字", "Signatures required")], ["stampRequired", text("要求盖章", "Institutional stamp required")], ["sharedFileAllowed", text("官方明确允许多个要求合并为一个文件", "Official source explicitly allows a combined file")] ] as const).map(([key, label]) => <label key={key}><input type="checkbox" checked={editing.declaration?.[key] ?? false} onChange={event => patchDeclaration({ [key]: event.target.checked })} />{label}</label>)}
      <label>{text("官方模板地址（可选）", "Official template URL (optional)")}<input type="url" value={editing.declaration?.templateUrl ?? ""} onChange={event => patchDeclaration({ templateUrl: event.target.value || null })} /></label>
      <label>{text("核验依据与调整说明", "Verification basis and adjustment note")}<textarea required minLength={4} value={editing.declaration?.authorNote ?? ""} onChange={event => patchDeclaration({ authorNote: event.target.value })} /></label>
      <p>{text("保存后相关材料和确认状态将重新核对，原文件与历史记录保留。", "Saving rechecks the affected materials and confirmations. Original files and history are retained.")}</p>
      <div><button type="submit" className="primary-button" disabled={locked}>{busy ? text("正在保存…", "Saving…") : text("保存核验结果", "Save verification")}</button><button type="button" className="text-button" disabled={busy} onClick={() => setEditing(null)}>{text("取消", "Cancel")}</button></div>
    </form> : null}
    {error ? <p role="alert">{localizeBackendText(locale, error)}</p> : null}
  </section>;
}
