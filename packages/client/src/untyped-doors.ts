// The doors the kernel does not yet serve at all (finding M1, narrowed by L4 which generates the contract from the kernel SOURCE:
// docs/handover/2026-10-18-requests-to-the-massrelay-terminal.md request 1.1). At kernel 27b4681f0 three remain; the other eight were only missing from the published file.
// client.test.ts fails when the vendored full contract gains one of these, so the list shrinks the day the kernel serves it.
export const UNTYPED_DOORS = [] as const; // 2026-10-19: empty — the kernel serves every door the brief named after its batch s (main 0d2e72984); the test turns red again if one disappears
