#!/usr/bin/env node
// Pins the kernel facts the pack validator needs (block kinds, people kinds) so the repo checks without massrelay on disk.
// Reads MR_KERNEL_ROOT (a checkout of the kernel; required to re-pin); read-only on the kernel.
import { readdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs'; import { join, dirname } from 'node:path'; import { fileURLToPath } from 'node:url'; import { execFileSync } from 'node:child_process';
const KERNEL = process.env.MR_KERNEL_ROOT ?? '';
const OUT = join(dirname(fileURLToPath(import.meta.url)), 'kernel-pins.json');
// D264.1: five block kinds that are not yet in the kernel's pack; removed from this list when they merge (the union is sorted and deduped).
const D264_1 = ['chart', 'timeline', 'checklist', 'board', 'decision'];
if (!existsSync(join(KERNEL, 'packs/control/blocks'))) { console.log('NOTE kernel not on disk — pins unchanged'); process.exit(0); }
const blocks = readdirSync(join(KERNEL, 'packs/control/blocks')).filter((f) => f.endsWith('.json')).map((f) => f.slice(0, -5));
const gk = JSON.parse(readFileSync(join(KERNEL, 'packs/control/people/goal-kinds.json'), 'utf8').replace(/^﻿/, ''));
const list = Array.isArray(gk) ? gk : (gk.kinds ?? gk.goal_kinds ?? Object.values(gk).find(Array.isArray));
const people = list.map((k) => (typeof k === 'string' ? k : k.id));
// nudge channels: every /api/channels/<name>/ door in the kernel router, plus the two non-chat channels the kernel serves (push, mail)
const router = readFileSync(join(KERNEL, 'crates/mr-server/src/lib.rs'), 'utf8');
const channels = [...new Set([...router.matchAll(/\/api\/channels\/([a-z0-9_]+)\//g)].map((m) => m[1]).concat(['push', 'mail']))].sort();
// verifier kinds: the goal checkers (checkers.rs) plus professional sign-off and the person's attestation (spec 02 section 1)
const verifiers = ['key_result', 'eval_suite', 'script', 'professional_signoff', 'attestation'];
// the locales the locale layer (packs/atingo/i18n) may name; ours, not the kernel's (L13)
const LOCALES = ['id', 'en'];
// the country layer (L7): the currencies a country's price may use are OURS to state until the kernel's row carries one (handover 1.14); the rails come from the kernel's row
const CURRENCIES = { ID: ['IDR'] };
const countries = {};
for (const iso of Object.keys(CURRENCIES)) { const f = join(KERNEL, 'packs/control/countries', `${iso}.json`); if (!existsSync(f)) continue; const c = JSON.parse(readFileSync(f, 'utf8').replace(/^﻿/, '')); countries[iso] = { currencies: CURRENCIES[iso], rails: [...c.rails], receipt_fields: [...c.receipt_fields], confirmed: c.confirmed === true }; }
const sha = execFileSync('git', ['-C', KERNEL, 'rev-parse', '--short', 'HEAD'], { encoding: 'utf8' }).trim();
const pins = { kernel_sha: sha, block_kinds: [...new Set([...blocks, ...D264_1])].sort(), people_kinds: [...new Set(people)].sort(), nudge_channels: channels, verifier_kinds: verifiers, locales: LOCALES, countries };
writeFileSync(OUT, JSON.stringify(pins, null, 2) + '\n');
console.log(`pins: kernel ${sha} · ${pins.block_kinds.length} block kinds · ${pins.people_kinds.length} people kinds · ${pins.nudge_channels.length} nudge channels · ${pins.verifier_kinds.length} verifier kinds`);
