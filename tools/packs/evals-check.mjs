// The offline shape gate for a persona eval set (L15; spec 02 section 5). It checks the SHAPE the kernel's golden sets have
// (id, prompt, mustContain, mustNotContain) plus our intent and why; it never calls a model.
export const MIN_CASES = 12, MIN_PER_INTENT = 2;
export const INTENTS = ['first_step', 'evidence_ask', 'no_skip', 'protected_ground', 'no_guarantee'];
export function checkEvalSet(set) {
  if (!Array.isArray(set)) return ['the eval set is not an array'];
  const problems = [], seen = new Set(), count = Object.fromEntries(INTENTS.map((i) => [i, 0]));
  if (set.length < MIN_CASES) problems.push(`${set.length} cases (needs ${MIN_CASES})`);
  set.forEach((c, i) => {
    const at = `case ${i} "${c?.id}"`;
    for (const k of ['id', 'prompt', 'why']) if (typeof c?.[k] !== 'string' || !c[k]) problems.push(`${at}: ${k} is missing`);
    if (!Array.isArray(c?.mustContain) || c.mustContain.length === 0) problems.push(`${at}: mustContain is empty or missing`);
    if (!Array.isArray(c?.mustNotContain)) problems.push(`${at}: mustNotContain is missing`);
    if (!INTENTS.includes(c?.intent)) problems.push(`${at}: intent "${c?.intent}" is not one of ${INTENTS.join(', ')}`); else count[c.intent]++;
    if (seen.has(c?.id)) problems.push(`${at}: duplicate id`); seen.add(c?.id);
  });
  for (const i of INTENTS) if (count[i] < MIN_PER_INTENT) problems.push(`intent ${i} appears ${count[i]} times (needs ${MIN_PER_INTENT})`);
  return problems;
}
