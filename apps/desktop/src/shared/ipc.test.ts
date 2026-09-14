import { beforeEach, expect, it, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());
const listenMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));

import { api } from "./ipc";

const project = {
  id: "project",
  revision: 3,
} as Parameters<typeof api.updateFacts>[0];
const facts = {
  sourceHash: "source",
} as Parameters<typeof api.updateFacts>[1];

beforeEach(() => {
  invokeMock.mockReset();
  listenMock.mockReset().mockResolvedValue(vi.fn());
});

it("reuses a requestId after an uncertain failure and rotates it after success", async () => {
  invokeMock
    .mockRejectedValueOnce(new Error("transport interrupted"))
    .mockResolvedValueOnce(project)
    .mockResolvedValueOnce(project);

  await expect(api.updateFacts(project, facts)).rejects.toMatchObject({
    code: "UNKNOWN",
  });
  await api.updateFacts(project, facts);
  const firstRequestId = invokeMock.mock.calls[0][1].requestId;
  expect(firstRequestId).toBe(invokeMock.mock.calls[1][1].requestId);
  expect(firstRequestId).toMatch(
    /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
  );

  await api.updateFacts(project, facts);
  expect(invokeMock.mock.calls[2][1].requestId).not.toBe(firstRequestId);
});

it("waits for a dispatched job event and returns its typed result", async () => {
  const queued = {
    schemaVersion: 1,
    id: crypto.randomUUID(),
    projectId: project.id,
    requestId: crypto.randomUUID(),
    operation: "update_document_facts",
    status: "queued",
    cancelRequested: false,
    events: [
      {
        seq: 1,
        phase: "update_document_facts",
        status: "queued",
        createdAtUnixMs: 1,
      },
    ],
    updatedAtUnixMs: 1,
  } as const;
  const completed = {
    ...queued,
    status: "succeeded" as const,
    result: project,
    events: [
      ...queued.events,
      {
        seq: 2,
        phase: "update_document_facts",
        status: "running" as const,
        createdAtUnixMs: 2,
      },
      {
        seq: 3,
        phase: "update_document_facts",
        status: "succeeded" as const,
        createdAtUnixMs: 3,
      },
    ],
    updatedAtUnixMs: 3,
  };
  invokeMock.mockResolvedValueOnce(queued);
  listenMock.mockImplementationOnce(
    async (_eventName: string, handler: (event: { payload: unknown }) => void) => {
      window.setTimeout(() => handler({ payload: completed }), 0);
      return vi.fn();
    },
  );

  await expect(api.updateFacts(project, facts)).resolves.toBe(project);
  expect(listenMock).toHaveBeenCalledWith(
    "manuscriptdock://job",
    expect.any(Function),
  );
  expect(invokeMock).toHaveBeenCalledTimes(1);
});

it("rotates the requestId after a known terminal job failure", async () => {
  const failedRequestId = crypto.randomUUID();
  invokeMock
    .mockResolvedValueOnce({
      schemaVersion: 1,
      id: failedRequestId,
      projectId: project.id,
      requestId: failedRequestId,
      operation: "update_document_facts",
      status: "needs_input",
      cancelRequested: false,
      events: [
        {
          seq: 1,
          phase: "update_document_facts",
          status: "needs_input",
          createdAtUnixMs: 1,
          error: {
            code: "CONTEXT_CHANGED",
            retryable: true,
            diagnosticId: "known-failure",
          },
        },
      ],
      updatedAtUnixMs: 1,
    })
    .mockResolvedValueOnce(project);

  await expect(api.updateFacts(project, facts)).rejects.toMatchObject({
    code: "CONTEXT_CHANGED",
  });
  const firstRequestId = invokeMock.mock.calls[0][1].requestId;
  await api.updateFacts(project, facts);
  expect(invokeMock.mock.calls[1][1].requestId).not.toBe(firstRequestId);
});
