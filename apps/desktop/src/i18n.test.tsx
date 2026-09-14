import { localize, localizeBackendText } from "./i18n";
import { expect, it } from "vitest";
it("keeps stable errors bilingual", () => {
  expect(localizeBackendText("zh-CN", "FORMAT_UNSUPPORTED")).toContain("DOCX");
  expect(localizeBackendText("en", "FORMAT_UNSUPPORTED")).toContain("DOCX");
  expect(localizeBackendText("zh-CN", "FORMAT_UNSUPPORTED")).toContain("PDF");
  expect(localizeBackendText("en", "FORMAT_UNSUPPORTED")).toContain("PDF");
  expect(localizeBackendText("zh-CN", "PDF_ENCRYPTED")).toContain("加密");
  expect(localizeBackendText("en", "PDF_ENCRYPTED")).toContain("encrypted");
  expect(localizeBackendText("zh-CN", "EDITABLE_MANUSCRIPT_REQUIRED")).toContain("可编辑");
  expect(localizeBackendText("en", "EDITABLE_MANUSCRIPT_REQUIRED")).toContain("editable");
  expect(localizeBackendText("zh-CN", "RECENT_INDEX_INVALID")).toContain("未被删除");
  expect(localizeBackendText("en", "RECENT_INDEX_INVALID")).toContain("No manuscript");
});
it("explains request conflicts in both locales", () => {
  expect(localizeBackendText("zh-CN", "REQUEST_ID_CONFLICT")).toContain("请求");
  expect(localizeBackendText("en", "REQUEST_ID_CONFLICT")).toContain("request");
});
it("explains interrupted job records in both locales", () => {
  expect(localizeBackendText("zh-CN", "JOB_RECORD_INVALID")).toContain("任务记录");
  expect(localizeBackendText("en", "JOB_RECORD_INVALID")).toContain("job record");
  expect(localizeBackendText("zh-CN", "JOB_CANCELLED")).toContain("取消");
  expect(localizeBackendText("en", "JOB_CANCELLED")).toContain("cancelled");
  expect(localizeBackendText("zh-CN", "JOB_INTERRUPTED")).toContain("后台任务");
  expect(localizeBackendText("en", "JOB_INTERRUPTED")).toContain("background job");
});
it("explains symbolic-link rejection in both locales", () => {
  expect(localizeBackendText("zh-CN", "INPUT_SYMLINK_NOT_ALLOWED")).toContain("符号链接");
  expect(localizeBackendText("en", "INPUT_SYMLINK_NOT_ALLOWED")).toContain("symbolic links");
  expect(localizeBackendText("zh-CN", "SNAPSHOT_CHANGED")).toContain("哈希");
  expect(localizeBackendText("en", "SNAPSHOT_CHANGED")).toContain("hash");
  expect(localizeBackendText("zh-CN", "MATERIAL_KIND_INVALID")).toContain("附件用途");
  expect(localizeBackendText("en", "MATERIAL_KIND_INVALID")).toContain("attachment purpose");
  expect(localizeBackendText("zh-CN", "ANONYMITY_UNVERIFIED")).toContain("匿名稿");
  expect(localizeBackendText("en", "ANONYMITY_UNVERIFIED")).toContain("author name");
});
it("explains safe project-folder failures in both locales", () => {
  expect(localizeBackendText("zh-CN", "PROJECT_FOLDER_UNAVAILABLE")).toContain("原项目文件夹");
  expect(localizeBackendText("en", "PROJECT_FOLDER_UNAVAILABLE")).toContain("original project folder");
  expect(localizeBackendText("zh-CN", "FOLDER_BINDING_CONFLICT")).toContain("错误位置");
  expect(localizeBackendText("en", "FOLDER_BINDING_CONFLICT")).toContain("wrong location");
  expect(localizeBackendText("zh-CN", "PROJECT_FOLDER_NOT_BOUND")).toContain("保存位置");
  expect(localizeBackendText("en", "PROJECT_FOLDER_NOT_BOUND")).toContain("save location");
});
it("selects the requested locale", () => {
  expect(localize("zh-CN", "本机", "Local")).toBe("本机");
  expect(localize("en", "本机", "Local")).toBe("Local");
});
