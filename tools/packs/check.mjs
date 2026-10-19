#!/usr/bin/env node
// The pack validator (L1; L3 moves the same rules into the atingo-path crate). Schema via Ajv, then the cross rules in plain code.
import { readdirSync, readFileSync, existsSync } from 'node:fs'; import { join, dirname, resolve } from 'node:path'; import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { checkEvalSet } from './evals-check.mjs';
import Ajv2020 from 'ajv/dist/2020.js'; import addFormats from 'ajv-formats';

const HERE = dirname(fileURLToPath(import.meta.url)); const REPO = resolve(HERE, '..', '..');
const PROTECTED = new Set('gender sex male female men women age old young elderly health sick healthy disabled disability religion faith faithful faithless nationality race ethnic orientation'.split(' '));
const load = (p) => JSON.parse(readFileSync(p, 'utf8').replace(/^﻿/, ''));
const schema = (n) => load(join(HERE, 'schemas', `${n}.schema.json`));
const jsonFiles = (dir) => (existsSync(dir) ? readdirSync(dir).filter((f) => f.endsWith('.json')).sort().map((f) => [f, join(dir, f)]) : []);
// a protected ground is a whole token of an id or key (split on - _ and space), compared in lower case
const grounds = (s) => String(s).toLowerCase().split(/[\s_-]+/).filter((t) => PROTECTED.has(t));
function keysDeep(v, out = []) { if (Array.isArray(v)) v.forEach((x) => keysDeep(x, out)); else if (v && typeof v === 'object') for (const [k, x] of Object.entries(v)) { out.push(k); keysDeep(x, out); } return out; }

const F = (code, path, message) => ({ code, path, message });

// the cross rules over one path file; the Rust crate (atingo_path::validate) implements the same codes and the fixtures prove they agree
function crossRules(name, p, kindIds, blocks, people, errors, warnings) {
  const ns = p.nodes ?? [], seen = new Set(), ids = new Set(ns.map((n) => n.id));
  if (!ids.has(p.root)) errors.push(F('root_missing', name, `root "${p.root}" is not a node`));
  for (const n of ns) {
    if (seen.has(n.id)) errors.push(F('duplicate_node_id', name, `node id "${n.id}" appears more than once`)); seen.add(n.id);
    if (!kindIds.has(n.goal_kind)) errors.push(F('goal_kind_missing', name, `node "${n.id}" goal_kind "${n.goal_kind}" is not in goal-kinds/`));
    for (const b of n.blocks_allowed ?? []) if (!blocks.has(b)) errors.push(F('block_kind_unknown', name, `node "${n.id}" blocks_allowed "${b}" is not a kernel block kind`));
    const nsrc = (n.sources ?? []).length;
    if (p.tier === 'platform' && nsrc < 2) {
      if (p.price_ref) errors.push(F('sources_missing_when_priced', name, `platform node "${n.id}" has fewer than two sources (${nsrc}) and price_ref is set`));
      else warnings.push(F('sources_missing_unpriced', name, `platform node "${n.id}" has fewer than two sources (${nsrc}); required before price_ref is set`));
    }
  }
  for (const e of p.edges ?? []) {
    if (!ids.has(e.from)) errors.push(F('edge_node_missing', name, `edge "${e.from}" -> "${e.to}" names a missing node "${e.from}"`));
    if (!ids.has(e.to)) errors.push(F('edge_node_missing', name, `edge "${e.from}" -> "${e.to}" names a missing node "${e.to}"`));
  }
  for (const [nid, kinds] of Object.entries(p.people_kinds_by_node ?? {})) {
    if (!ids.has(nid)) errors.push(F('people_node_missing', name, `people_kinds_by_node names a missing node "${nid}"`));
    for (const k of kinds) if (!people.has(k)) errors.push(F('people_kind_unknown', name, `node "${nid}" people kind "${k}" is not in the kernel's goal-kinds.json`));
  }
  if (p.certificate && p.tier !== 'platform') errors.push(F('certificate_on_non_platform', name, `certificate on tier ${p.tier} (a certificate belongs to platform only)`));
  // prerequisite cycle: DFS in node order over prerequisite edges only (a branch loop is allowed); report the first cycle found
  const next = new Map(); for (const e of p.edges ?? []) if (e.kind === 'prerequisite') (next.get(e.from) ?? next.set(e.from, []).get(e.from)).push(e.to);
  const state = new Map(), stack = []; let cyc = null;
  const dfs = (n) => { if (cyc) return; if (state.get(n) === 1) { cyc = [...stack.slice(stack.indexOf(n)), n]; return; } if (state.get(n) === 2) return;
    state.set(n, 1); stack.push(n); for (const m of next.get(n) ?? []) dfs(m); stack.pop(); state.set(n, 2); };
  for (const n of ns) dfs(n.id);
  if (cyc) errors.push(F('prerequisite_cycle', name, `prerequisite cycle: ${cyc.join(' -> ')}`));
  // reachability from the root over all edges (skipped when the root itself is missing: that is already one error)
  if (ids.has(p.root)) {
    const adj = new Map(); for (const e of p.edges ?? []) (adj.get(e.from) ?? adj.set(e.from, []).get(e.from)).push(e.to);
    const reach = new Set([p.root]), q = [p.root];
    while (q.length) for (const m of adj.get(q.shift()) ?? []) if (!reach.has(m)) { reach.add(m); q.push(m); }
    for (const id of [...ids].sort()) if (!reach.has(id)) errors.push(F('node_unreachable_from_root', name, `node "${id}" is not reachable from root "${p.root}"`));
  }
}


// ---- the loop rules (L15): persona, lesson templates, follow-up, evidence. The Rust crate (atingo_path::validate_loop) has the same codes.
// The forbidden-instruction check is deliberately regex-free so the Rust twin can match it exactly: tokens of [a-z0-9]+, sentences split on . ! ? ; and newlines.
const words = (s) => s.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean);
export function forbiddenInstruction(text) {
  const found = [];
  if (/<\/?[a-z]+>/i.test(text)) found.push('markup');
  for (const sentence of text.split(/[.!?;\n]+/)) {
    const w = words(sentence);
    const at = (pred, from = 0) => w.findIndex((t, i) => i >= from && pred(t));
    const m = at((t) => ['mark', 'marks', 'marked', 'marking'].includes(t));
    if (m >= 0 && at((t) => ['done', 'complete', 'completed', 'finished'].includes(t), m + 1) >= 0) found.push('mark a step done');
    if (w.some((t) => t.startsWith('promis') || t.startsWith('guarant'))) found.push('promise or guarantee');
    if (w.some((t) => t === 'rating' || t === 'ratings')) found.push('rating');
    for (let i = 0; i + 2 < w.length; i++) if (['score', 'grade', 'rate', 'rank'].includes(w[i]) && w[i + 1] === 'the' && ['learner', 'person', 'user', 'student'].includes(w[i + 2])) found.push(`${w[i]} the ${w[i + 2]}`);
    const d = at((t) => ['infer', 'derive', 'deduce', 'guess', 'determine'].some((v) => t.startsWith(v)));
    if (d >= 0 && at((t) => PROTECTED.has(t), d + 1) >= 0) found.push('derive a protected ground');
  }
  return [...new Set(found)];
}
// the kernel's RRULE subset (lifecycle.rs parse_recurrence): FREQ required (4 values), INTERVAL 1..366, COUNT >= 1, nothing else
export function rruleOk(rule) {
  const body = rule.trim().replace(/^RRULE:/, ''); let freq = false;
  for (const part of body.split(';').filter(Boolean)) {
    const i = part.indexOf('='); if (i < 0) return false;
    const k = part.slice(0, i).toUpperCase(), v = part.slice(i + 1);
    if (k === 'FREQ') { if (!['DAILY', 'WEEKLY', 'MONTHLY', 'YEARLY'].includes(v.toUpperCase())) return false; freq = true; }
    else if (k === 'INTERVAL') { if (!/^[0-9]+$/.test(v) || +v < 1 || +v > 366) return false; }
    else if (k === 'COUNT') { if (!/^[0-9]+$/.test(v) || +v < 1) return false; }
    else return false;
  }
  return freq;
}
export const MIN_EVAL_CASES = 12;
function loopRules(path, name, d, kinds, pins, errors) {
  const { personas, evalSets, templates, followUp, evidence } = d; const ns = path.nodes ?? [];
  if (!path.persona) errors.push(F('persona_missing', name, `path "${path.id}" names no persona`));
  else {
    const per = personas.get(path.persona);
    if (!per) errors.push(F('persona_unknown', name, `path "${path.id}" persona "${path.persona}" has no file in personas/`));
    else {
      for (const f of forbiddenInstruction(per.taskTemplate ?? '')) errors.push(F('persona_forbidden_instruction', `personas/${per.key}.json`, `taskTemplate of "${per.key}" instructs a forbidden act: ${f}`));
      const n = evalSets.get(per.evalSetRef)?.length ?? 0;
      if (n < MIN_EVAL_CASES) errors.push(F('eval_set_too_small', `personas/evals/${per.evalSetRef}.json`, `eval set "${per.evalSetRef}" has ${n} cases (needs ${MIN_EVAL_CASES})`));
    }
  }
  for (const n of ns) if (!templates.has(n.lesson_template)) errors.push(F('lesson_template_missing', name, `node "${n.id}" lesson_template "${n.lesson_template}" has no file in lesson-templates/`));
  const people = path.people_kinds_by_node ?? {};
  for (const n of ns) {
    const fu = followUp?.nodes?.[n.id];
    if (!fu) errors.push(F('followup_missing', `follow-up/${path.id}.json`, `node "${n.id}" has no follow-up row`));
    else {
      if (!rruleOk(fu.rrule)) errors.push(F('followup_rrule_invalid', `follow-up/${path.id}.json`, `node "${n.id}" rrule "${fu.rrule}" is outside the kernel subset (FREQ=DAILY|WEEKLY|MONTHLY|YEARLY, INTERVAL, COUNT)`));
      for (const u of fu.nudges ?? []) if (!pins.nudge_channels.includes(u.channel)) errors.push(F('followup_channel_unknown', `follow-up/${path.id}.json`, `node "${n.id}" nudge channel "${u.channel}" is not a kernel channel`));
      if (people[n.id] && !people[n.id].includes(fu.escalate?.to)) errors.push(F('people_kind_unknown', `follow-up/${path.id}.json`, `node "${n.id}" escalate.to "${fu.escalate?.to}" is not one of its people kinds`));
    }
    const ev = evidence?.nodes?.[n.id];
    if (!ev) { errors.push(F('evidence_binding_missing', `evidence/${path.id}.json`, `node "${n.id}" has no evidence binding`)); continue; }
    const file = `evidence/${path.id}.json`, kind = kinds.get(n.goal_kind);
    if (!pins.verifier_kinds.includes(ev.verifier)) errors.push(F('evidence_verifier_unknown', file, `node "${n.id}" verifier "${ev.verifier}" is not a kernel verifier kind`));
    if (kind && !kind.attestation_fallback && (ev.verifier === 'attestation' || ev.fallback === 'attestation')) errors.push(F('evidence_verifier_unknown', file, `node "${n.id}" uses attestation but goal kind "${kind.id}" has attestation_fallback false`));
    const c = ev.binding?.connector_id;
    if (c && !(ev.sources_of_truth ?? []).includes(c)) errors.push(F('evidence_binding_outside_sources', file, `node "${n.id}" binding connector "${c}" is not in its sources_of_truth`));
  }
}


// ---- the review rules (L20, D17): packs/atingo/review/<path_id>.json. The Rust crate (atingo_path::validate_loop) has the same codes.
// Opt-in by folder: a pack root with no review/ is silent; with it, a node with no policy is a warning. desired_retention must lie strictly inside (0.5, 0.99).
function reviewRules(path, name, rv, layer, errors, warnings) {
  if (!layer) return; const file = `review/${path.id}.json`;
  for (const n of path.nodes ?? []) {
    const row = rv?.nodes?.[n.id];
    if (!row) { warnings.push(F('review_policy_missing', file, `node "${n.id}" has no review policy`)); continue; }
    if (row.engine !== 'fsrs') errors.push(F('review_engine_unknown', file, `node "${n.id}" review engine "${row.engine}" is not "fsrs"`));
    if (!(row.desired_retention > 0.5 && row.desired_retention < 0.99)) errors.push(F('review_retention_out_of_range', file, `node "${n.id}" desired_retention ${row.desired_retention} is outside (0.5, 0.99)`));
  }
}

// ---- the locale rules (L13, spec 01 section 5): packs/atingo/i18n/<path_id>.<locale>.json. The Rust crate (atingo_path::validate_i18n) has the same codes.
// The law: the locale layer never adds or removes a step (the count equals the template's levels[0].steps), and no string is empty.
function i18nRules(name, l, paths, templates, locales, errors) {
  const path = paths.get(l.path_id);
  if (!path) { errors.push(F('i18n_path_unknown', name, `locale file names path "${l.path_id}" that has no file in paths/`)); return; }
  if (!locales.has(l.locale)) errors.push(F('i18n_locale_unsupported', name, `locale "${l.locale}" is not in kernel-pins.json locales`));
  const ids = new Set(path.nodes.map((n) => n.id));
  for (const id of Object.keys(l.nodes)) if (!ids.has(id)) errors.push(F('i18n_node_unknown', name, `locale file names node "${id}" that path "${path.id}" does not have`));
  for (const n of path.nodes) {
    const e = l.nodes[n.id];
    if (!e) { errors.push(F('i18n_node_missing', name, `path node "${n.id}" has no entry in the ${l.locale} locale file`)); continue; }
    if (!e.title.trim()) errors.push(F('i18n_empty_string', name, `node "${n.id}" title is empty`));
    e.lesson.steps.forEach((s, i) => { for (const k of ['prompt', 'evidence_hint']) if (!s[k].trim()) errors.push(F('i18n_empty_string', name, `node "${n.id}" step ${i + 1} ${k} is empty`)); });
    const want = templates.get(n.lesson_template)?.levels?.[0]?.steps?.length;
    if (want !== undefined && e.lesson.steps.length !== want) errors.push(F('i18n_steps_count_mismatch', name, `node "${n.id}" has ${e.lesson.steps.length} steps in ${l.locale}, the template "${n.lesson_template}" has ${want}`));
  }
}

// ---- the graph rules (L16, spec 03 §3, §4.1): packs/atingo/graph/. The Rust crate (atingo_path::validate_graph) has the same codes and the fixtures prove they agree.
// Tokens are [a-z0-9]+ so the Rust twin matches exactly: "risk-free" is the token run risk free, "cures" never matches "secures".
const CLAIMS = [['guaranteed'], ['will', 'earn'], ['cures'], ['risk', 'free']];
export const claimVerbs = (text) => { const w = words(text); return CLAIMS.filter((c) => w.some((_, i) => c.every((t, k) => w[i + k] === t))).map((c) => c.join(' ')); };
const today = () => process.env.ATINGO_TODAY || new Date().toISOString().slice(0, 10);
function graphRules(g, errors, warnings) {
  const sectors = new Map(g.sectors.map((s) => [s.id, s])), comps = new Map(g.competencies.map((c) => [c.id, c])), sources = new Map(g.sources.map((r) => [r.url, r]));
  const used = new Set(), items = new Set(g.items.flatMap(([, f]) => f.items.map((i) => i.id)));
  for (const [name, o] of g.occupations) {
    if (!sectors.has(o.part_of)) errors.push(F('graph_parent_missing', name, `occupation "${o.id}" part_of "${o.part_of}" is not a sector`));
    for (const t of o.top) { if (!comps.has(t.comp)) errors.push(F('graph_parent_missing', name, `occupation "${o.id}" names competency "${t.comp}" that does not exist`)); used.add(t.comp); }
  }
  for (const [name, c] of g.competencyRows) {
    if (!sectors.has(c.part_of)) errors.push(F('graph_parent_missing', name, `competency "${c.id}" part_of "${c.part_of}" is not a sector`));
    for (const r of c.requires ?? []) if (!comps.has(r)) errors.push(F('graph_parent_missing', name, `competency "${c.id}" requires "${r}" that does not exist`));
    for (const oc of c.occupations ?? []) if (g.occIds.has(oc)) used.add(c.id); else errors.push(F('graph_parent_missing', name, `competency "${c.id}" names occupation "${oc}" that does not exist`));
  }
  for (const c of g.competencyRows.map(([, x]) => x)) if (!used.has(c.id)) errors.push(F('graph_competency_orphan', 'competencies', `competency "${c.id}" is in no occupation (and no path teaches it)`));
  // requires cycle across all competency files: DFS in sorted id order, report the first cycle found
  const state = new Map(), stack = []; let cyc = null;
  const dfs = (n) => { if (cyc || !comps.has(n)) return; if (state.get(n) === 1) { cyc = [...stack.slice(stack.indexOf(n)), n]; return; } if (state.get(n) === 2) return;
    state.set(n, 1); stack.push(n); for (const m of comps.get(n).requires ?? []) dfs(m); stack.pop(); state.set(n, 2); };
  for (const id of [...comps.keys()].sort()) dfs(id);
  if (cyc) errors.push(F('graph_requires_cycle', 'competencies', `requires cycle: ${cyc.join(' -> ')}`));
  const now = today();
  for (const [name, f] of g.items) {
    const comp = comps.get(f.competency), sector = comp && sectors.get(comp.part_of), reg = !!sector?.regulated;
    if (!comp) errors.push(F('graph_parent_missing', name, `items file names competency "${f.competency}" that does not exist`));
    for (const it of f.items) {
      const urls = new Set(it.sources.map((x) => x.url));
      if (urls.size < 2) errors.push(F('graph_item_sources_lt_2', name, `item "${it.id}" has ${urls.size} distinct source(s) (needs 2)`));
      for (const x of it.sources) { const r = sources.get(x.url); if (!r || r.sha256 !== x.sha256) errors.push(F('graph_item_source_unfetched', name, `item "${it.id}" source ${x.url} has no matching record (url and sha256) in sources/`)); }
      if (it.kind === 'figure' && !it.valid_until) errors.push(F('graph_figure_without_valid_until', name, `figure item "${it.id}" has no valid_until`));
      if (it.valid_until && it.valid_until < now) warnings.push(F('graph_valid_until_past', name, `item "${it.id}" valid_until ${it.valid_until} is past (${now})`));
      if (reg && it.badge !== 'information_not_advice') errors.push(F('graph_regulated_unbadged', name, `item "${it.id}" is under regulated sector "${sector.id}" without badge information_not_advice`));
      if (reg) for (const v of claimVerbs(it.text)) errors.push(F('graph_regulated_claim_verb', name, `item "${it.id}" says "${v}" in a regulated sector`));
    }
  }
  for (const [name, f] of g.lessons) for (const l of f.lessons) for (const st of l.steps) if (!items.has(st.item)) errors.push(F('graph_parent_missing', name, `lesson "${l.id}" step names item "${st.item}" that does not exist`));
}
function loadGraph(root, ajv, errors) {
  const dir = join(root, 'graph'); if (!existsSync(dir)) return null;
  const v = (n) => ajv.compile(schema(n)); const V = { sectors: v('graph-sectors'), occupations: v('graph-occupations'), competencies: v('graph-competencies'), items: v('graph-items'), lessons: v('graph-lessons'), sources: v('graph-sources') };
  const one = (file, name, validate) => { try { const d = load(file); if (!validate(d)) for (const e of validate.errors) errors.push(F('schema', name, `schema ${e.instancePath || '/'} ${e.message}${e.params?.additionalProperty ? ` "${e.params.additionalProperty}"` : ''}`)); return d; } catch (e) { errors.push(F('load_error', name, e.message)); return null; } };
  const many = (sub, val) => jsonFiles(join(dir, sub)).map(([f, p]) => [`graph/${sub}/${f}`, one(p, `graph/${sub}/${f}`, val)]).filter(([, d]) => d);
  const lessonFiles = existsSync(join(dir, 'lessons')) ? readdirSync(join(dir, 'lessons')).sort().flatMap((p) => jsonFiles(join(dir, 'lessons', p)).map(([f, fp]) => [`graph/lessons/${p}/${f}`, one(fp, `graph/lessons/${p}/${f}`, V.lessons)]).filter(([, d]) => d)) : [];
  const sj = jsonFiles(dir).find(([f]) => f === 'sectors.json'); const sectors = sj ? one(sj[1], 'graph/sectors.json', V.sectors) : null; if (!sectors) errors.push(F('load_error', 'graph/sectors.json', 'missing or unreadable'));
  const occupations = many('occupations', V.occupations), competencies = many('competencies', V.competencies), items = many('items', V.items), sources = many('sources', V.sources);
  if (errors.some((e) => e.path.startsWith('graph/') && (e.code === 'schema' || e.code === 'load_error'))) return { bad: true };
  const g = { sectors: sectors?.sectors ?? [], occupations: occupations.flatMap(([n, f]) => f.occupations.map((o) => ({ ...o, _f: n }))).map((o) => [o._f, o]), items,
    competencyRows: competencies.flatMap(([n, f]) => f.competencies.map((c) => [n, c])), competencies: competencies.flatMap(([, f]) => f.competencies), lessons: lessonFiles, sources: sources.map(([, r]) => r) };
  g.occIds = new Set(g.occupations.map(([, o]) => o.id));
  g.counts = { sectors: g.sectors.length, occupations: g.occupations.length, competencies: g.competencies.length, items: items.reduce((a, [, f]) => a + f.items.length, 0) };
  return g;
}

// ---- the price rules (L7, handover 1.13): packs/atingo/prices/<id>.json is the row the kernel's price book will hold. The Rust crate (atingo_path::validate_price) has the same codes.
// Whole rupiah in micro-units: amount_mu is a multiple of 1,000,000 and equals amount x 1,000,000. The country rows (pinned) say which currency and rails a country allows.
const MU = 1_000_000;
function priceRules(name, r, pins, errors) {
  const c = (pins.countries ?? {})[r.country];
  if (!c) { errors.push(F('price_country_unknown', name, `price country "${r.country}" has no country row in kernel-pins.json`)); return; }
  if (!c.currencies.includes(r.currency)) errors.push(F('price_currency_not_allowed', name, `price currency "${r.currency}" is not one the ${r.country} country rows allow (${c.currencies.join(', ')})`));
  if (!Number.isInteger(r.amount_mu) || r.amount_mu % MU !== 0 || r.amount_mu !== r.amount * MU) errors.push(F('price_amount_not_whole', name, `amount_mu ${r.amount_mu} is not a whole ${r.currency} unit in micro-units (amount ${r.amount} x ${MU})`));
  for (const rail of r.rails) if (!c.rails.includes(rail)) errors.push(F('price_rail_not_allowed', name, `price rail "${rail}" is not one the ${r.country} country rows allow (${c.rails.join(', ')})`));
}

// ATINGO_PATH_BIN names the built atingo-path binary: when set, ITS findings replace the JS cross rules (schema and motives stay here)
function delegate(bin, root, pinsFile, errors, warnings) {
  const r = spawnSync(bin, ['check', root, '--pins', pinsFile], { encoding: 'utf8' });
  let out; try { out = JSON.parse(r.stdout.trim().split('\n').pop()); } catch { out = null; }
  if (!out) { errors.push(F('load_error', 'atingo-path', `${bin} exited ${r.status}: ${(r.stderr || r.stdout || '').trim()}`)); return; }
  for (const f of out.findings) (f.severity === 'warning' ? warnings : errors).push(F(f.code, f.path, f.message));
}

export function validatePacks(root, pins, pinsFile = join(HERE, 'kernel-pins.json')) {
  const errors = [], warnings = [];
  const ajv = new Ajv2020({ allErrors: true }); addFormats(ajv);
  const vPersona = ajv.compile(schema('persona')), vEval = ajv.compile(schema('eval-set')), vTemplate = ajv.compile(schema('lesson-template')), vFollow = ajv.compile(schema('follow-up')), vEvidence = ajv.compile(schema('evidence')), vI18n = ajv.compile(schema('i18n')), vReview = ajv.compile(schema('review')), vPrice = ajv.compile(schema('prices'));
  const vGoal = ajv.compile(schema('goal-kinds')), vPath = ajv.compile(schema('path')), vMotives = ajv.compile(schema('motives'));
  const sch = (v, data, name) => { if (!v(data)) for (const e of v.errors) errors.push(F('schema', name, `schema ${e.instancePath || '/'} ${e.message}${e.params?.additionalProperty ? ` "${e.params.additionalProperty}"` : ''}`)); };
  const readAll = (sub, v) => jsonFiles(join(root, sub)).map(([f, p]) => { const name = `${sub}/${f}`; try { const d = load(p); sch(v, d, name); return [name, d]; } catch (e) { errors.push(F('load_error', name, e.message)); return null; } }).filter(Boolean);

  const goalFiles = readAll('goal-kinds', vGoal), pathFiles = readAll('paths', vPath), motiveFiles = readAll('motives', vMotives);
  const personaFiles = readAll('personas', vPersona), evalFiles = readAll('personas/evals', vEval), templateFiles = readAll('lesson-templates', vTemplate), followFiles = readAll('follow-up', vFollow), evidenceFiles = readAll('evidence', vEvidence), i18nFiles = readAll('i18n', vI18n), reviewFiles = readAll('review', vReview), reviewLayer = existsSync(join(root, 'review')), priceFiles = readAll('prices', vPrice);
  const loop = { personas: new Map(personaFiles.map(([, p]) => [p.key, p])), evalSets: new Map(evalFiles.map(([n, e]) => [n.split('/').pop().slice(0, -5), e])), templates: new Map(templateFiles.map(([, t]) => [t.id, t])) };
  let evalCases = 0;
  for (const [name, e] of evalFiles) { evalCases += e.length; for (const pr of checkEvalSet(e)) errors.push(F('eval_set_shape', name, pr)); }
  const kindMap = new Map(goalFiles.flatMap(([, g]) => g.kinds ?? []).map((k) => [k.id, k]));
  const kindIds = new Set(goalFiles.flatMap(([, g]) => (g.kinds ?? []).map((k) => k.id)));
  const blocks = new Set(pins.block_kinds), people = new Set(pins.people_kinds);
  const pathIds = new Set(pathFiles.map(([, p]) => p.id));
  let nodes = 0;

  for (const [name, g] of goalFiles) for (const k of g.kinds ?? []) {
    if (k.verifier === 'attestation' && k.attestation_fallback !== true) errors.push(F('attestation_fallback_missing', name, `kind "${k.id}" verifier attestation needs attestation_fallback true`));
  }
  const bin = process.env.ATINGO_PATH_BIN;
  const graph = loadGraph(root, ajv, errors); if (graph && !graph.bad && !bin) graphRules(graph, errors, warnings);
  for (const [name, p] of pathFiles) { nodes += (p.nodes ?? []).length; if (!bin) { crossRules(name, p, kindIds, blocks, people, errors, warnings);
    loopRules(p, name, { ...loop, followUp: followFiles.find(([, f]) => f.path_id === p.id)?.[1], evidence: evidenceFiles.find(([, f]) => f.path_id === p.id)?.[1] }, kindMap, pins, errors);
    reviewRules(p, name, reviewFiles.find(([, f]) => f.path_id === p.id)?.[1], reviewLayer, errors, warnings); } }
  const locales = new Set(pins.locales ?? []), pathMap = new Map(pathFiles.map(([, p]) => [p.id, p]));
  if (!bin) for (const [name, l] of i18nFiles) i18nRules(name, l, pathMap, loop.templates, locales, errors);
  const priceIds = new Set(priceFiles.map(([n]) => n.split('/').pop().slice(0, -5)));
  if (!bin) for (const [name, p] of pathFiles) if (p.price_ref && !priceIds.has(p.price_ref)) errors.push(F('price_ref_unresolved', name, `price_ref "${p.price_ref}" has no row in prices/`));
  if (!bin) for (const [name, r] of priceFiles) priceRules(name, r, pins, errors);
  if (bin) delegate(bin, root, pinsFile, errors, warnings);
  let motives = 0;
  for (const [name, m] of motiveFiles) for (const x of m.motives ?? []) {
    motives++;
    for (const g of grounds(x.id)) errors.push(F('protected_ground', name, `motive "${x.id}" id names protected ground "${g}"`));
    // keys at any depth, never the values of line or promise (the founder's words and the promise copy)
    for (const key of keysDeep(x)) for (const g of grounds(key)) errors.push(F('protected_ground', name, `motive "${x.id}" key "${key}" is a protected ground "${g}"`));
    if ('segment' in x) errors.push(F('motive_segment', name, `motive "${x.id}" has a segment field (targeting is declared_goal_only)`));
    for (const ep of x.entry_paths ?? []) if (!pathIds.has(ep)) errors.push(F('motive_path_missing', name, `motive "${x.id}" entry_paths names a missing path "${ep}"`));
  }
  return { errors, warnings, counts: { evalCases, personas: personaFiles.length, templates: templateFiles.length, kinds: goalFiles.reduce((a, [, g]) => a + (g.kinds?.length ?? 0), 0), paths: pathFiles.length, nodes, motives, locales: i18nFiles.length, prices: priceFiles.length, graph: graph?.counts ?? { sectors: 0, occupations: 0, competencies: 0, items: 0 } } };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const dir = process.argv[2] ? resolve(process.argv[2]) : join(REPO, 'packs/atingo');
  const pinsFile = join(HERE, 'kernel-pins.json'); const r = validatePacks(dir, load(pinsFile), pinsFile);
  if (r.errors.length) { console.error(r.errors.map((e) => `${e.code} ${e.path}: ${e.message}`).join('\n')); process.exit(1); }
  const c = r.counts; console.log(`packs ok: ${c.kinds} kinds · ${c.paths} paths · ${c.nodes} nodes · ${c.motives} motives · ${c.personas} personas · ${c.templates} templates · eval cases ${c.evalCases} · locales ${c.locales} · graph: ${c.graph.sectors} sectors · ${c.graph.occupations} occupations · ${c.graph.competencies} competencies · ${c.graph.items} items · ${r.warnings.length} warnings${process.env.ATINGO_PATH_BIN ? ' (rust)' : ''}`);
}
