// The page's half of "the big brain" (Rust: src-tauri/src/brain.rs). Starts
// runs and hands each one's events back to whoever asked, as a promise that
// settles with the final result.
//
// A result: { kind: 'result' | 'error', text, is_error, session, denials,
//             texts: [every text block], servers: [{ name, status }] }
'use strict';

// Background checks run on the smallest model, setup and requests on a
// stronger one: that's most of what keeps him light on the user's plan.
export const MODELS = {
  quick: 'claude-haiku-4-5-20251001',
  main: 'claude-sonnet-5',
};

// note(): instructions added to every run (how he talks, from personality.js)
export function createBrain(invoke, listen, { note = null } = {}) {
  const jobs = new Map();
  let seq = 0;
  let busy = 0;

  if (listen) {
    listen('brain', (e) => {
      const ev = e.payload;
      const j = jobs.get(ev.job);
      if (!j) return;
      if (ev.kind === 'init') j.servers = ev.servers;
      if (ev.kind === 'text') j.texts.push(ev.text);
      if (ev.session) j.session = ev.session;
      try { if (j.onEvent) j.onEvent(ev); } catch (err) { console.error(err); }
      if (ev.kind === 'result' || ev.kind === 'error') {
        jobs.delete(ev.job);
        busy = Math.max(0, busy - 1);
        j.resolve({ ...ev, session: ev.session || j.session, texts: j.texts, servers: j.servers || [] });
      }
    });
  }

  // prompt: what to do. extra: the job's instructions, added after the skill.
  // mode: 'auto' | 'plan'. alsoAllow: tools the user just confirmed.
  function run({ prompt, extra = '', model = MODELS.main, mode = 'auto', resume = null, alsoAllow = [], maxTurns = 30, onEvent } = {}) {
    const job = `j${++seq}-${Date.now()}`;
    const always = note ? note() : '';
    extra = [extra, always].filter((x) => x && x.trim()).join('\n\n');
    busy++;
    return new Promise((resolve) => {
      jobs.set(job, { resolve, onEvent, texts: [], servers: null, session: '' });
      invoke('brain_run', { job, prompt, extra, model, mode, resume, alsoAllow, maxTurns }).catch((err) => {
        jobs.delete(job);
        busy = Math.max(0, busy - 1);
        resolve({ kind: 'error', text: String(err), is_error: true, texts: [], servers: [], denials: [] });
      });
    });
  }

  return {
    run,
    status: (model = MODELS.quick) => invoke('brain_status', { model }),
    stop: () => invoke('brain_stop'),
    open: (what) => invoke('brain_open', { what }),
    get busy() { return busy > 0; },
  };
}

// The line he says for a reply: its first non-empty line, minus markdown.
export function spokenLine(text) {
  const first = String(text || '').split('\n').map((l) => l.trim()).find(Boolean) || '';
  return first.replace(/^[#>*\-\s]+/, '').replace(/[*_`]/g, '').slice(0, 200);
}

// setup's closing line: SUMMARY: {...}
export function readSummary(texts) {
  for (const t of [...texts].reverse()) {
    const m = String(t).match(/SUMMARY:\s*(\{[\s\S]*\})/);
    if (m) { try { return JSON.parse(m[1]); } catch {} }
  }
  return null;
}

// the setup stage a text block announces (STAGE 2/3), or 0
// The DRAFT: {...} line a request ends with when it wrote something for the
// user (the skill's request job). Returns { to, app, subject, body, link } or
// null; the JSON may wrap, so it's read by matching braces.
export function readDraft(texts) {
  for (const t of [...texts].reverse()) {
    const s = String(t || '');
    const at = s.lastIndexOf('DRAFT:');
    if (at < 0) continue;
    const open = s.indexOf('{', at);
    if (open < 0) continue;
    // copied as it's scanned, with real line breaks inside strings escaped:
    // a model often writes the body's newlines as newlines
    let depth = 0, inStr = false, esc = false, json = '';
    for (let i = open; i < s.length; i++) {
      const c = s[i];
      if (inStr) {
        if (esc) esc = false;
        else if (c === '\\') esc = true;
        else if (c === '"') inStr = false;
        json += c === '\n' ? '\\n' : c === '\t' ? '\\t' : c === '\r' ? '' : c;
        continue;
      }
      json += c;
      if (c === '"') inStr = true;
      else if (c === '{') depth++;
      else if (c === '}' && --depth === 0) {
        try {
          const d = JSON.parse(json);
          if (d && typeof d.body === 'string' && d.body.trim()) {
            return { to: String(d.to || ''), app: String(d.app || ''), subject: String(d.subject || ''), body: d.body, link: String(d.link || '') };
          }
        } catch {}
        break;
      }
    }
  }
  return null;
}
// the reply without its DRAFT line (that's for him, not to read out)
export function withoutDraft(text) {
  const s = String(text || '');
  const at = s.lastIndexOf('DRAFT:');
  return (at < 0 ? s : s.slice(0, at)).trim();
}

export function readStage(text) {
  const m = String(text || '').match(/STAGE\s+(\d)\s*\/\s*(\d)/i);
  return m ? Number(m[1]) : 0;
}

// what a denied send was going to do, in words he can show
export function describeSend(denial) {
  const tool = String(denial.tool || '').split('__').pop().replace(/_/g, ' ');
  const i = denial.input || {};
  const to = i.to || i.recipient || i.recipients || i.channel || i.channel_id || i.attendees || i.email || '';
  const subject = i.subject || i.summary || i.title || '';
  const body = i.body || i.text || i.message || i.content || i.description || '';
  const flat = (v) => (Array.isArray(v) ? v.map((x) => (typeof x === 'object' ? (x.email || x.name || JSON.stringify(x)) : x)).join(', ') : typeof v === 'object' ? JSON.stringify(v) : String(v));
  return {
    action: tool,
    to: flat(to),
    subject: flat(subject),
    body: flat(body).slice(0, 600),
  };
}
