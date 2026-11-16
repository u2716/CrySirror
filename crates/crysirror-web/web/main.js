// SPDX-License-Identifier: MPL-2.0
const $ = id => document.getElementById(id);
const status = (text, error = false) => {
  $('status').textContent = text;
  $('status-line').className = 'status' + (error ? ' error' : running ? ' running' : '');
};
let worker, audioContext, output, busy = false, running = false, paused = false, operation = false;
let down = 0, pressed = 0, pointer = [-1, -1], logical = [640, 360];
const keyBits = new Map([['Enter', 1], ['Space', 2], ['ArrowLeft', 4], ['ArrowRight', 8], ['ArrowUp', 16], ['ArrowDown', 32], ['Tab', 64], ['KeyW', 128], ['KeyA', 256], ['KeyS', 512], ['KeyD', 1024], ['ShiftLeft', 2048], ['ControlLeft', 4096], ['Escape', 1 << 19]]);
function controls(enabled) {
  for (const id of ['pause', 'save', 'load', 'fullscreen']) $(id).disabled = !enabled;
  $('viewport').classList.toggle('playing', enabled);
}
async function stop() {
  running = false; busy = false; operation = false; worker?.terminate(); worker = undefined;
  output?.disconnect(); if (audioContext) await audioContext.close(); audioContext = undefined; output = undefined;
  down = pressed = 0; paused = false; controls(false); $('pause').textContent = 'Pause';
}
// `extra`: {library: {entries, covers, size}} runs the shared launcher;
// {font} supplies the owner font registered as "default".
async function boot(project, names, bytes, saveKey, extra = {}) {
  await stop();
  if (!navigator.gpu || typeof OffscreenCanvas !== 'function') throw new Error('WebGPU and OffscreenCanvas are required. Open this host through localhost or HTTPS.');
  logical = extra.library ? extra.library.size : JSON.parse(project).canvas;
  audioContext = new AudioContext({ sampleRate: 48000 });
  const resume = audioContext.resume();
  await audioContext.audioWorklet.addModule('./audio.js'); await resume;
  output = new AudioWorkletNode(audioContext, 'crysirror-output', { outputChannelCount: [2] }); output.connect(audioContext.destination);
  const old = $('game'); const canvas = old.cloneNode(); old.replaceWith(canvas); canvas.width = logical[0]; canvas.height = logical[1]; bindInput(canvas);
  $('viewport').style.aspectRatio = `${logical[0]} / ${logical[1]}`;
  $('viewport').style.maxWidth = `${Math.max(logical[0], 960)}px`;
  const display = canvas.getContext('bitmaprenderer');
  if (!display) throw new Error('This browser does not support ImageBitmap presentation.');
  const session = new Worker('./worker.js', { type: 'module' });
  worker = session;
  session.onerror = event => { if (worker !== session) return; status(`Browser engine error: ${event.message}`, true); running = false; controls(false); };
  session.onmessage = ({ data }) => {
    if (worker !== session) { data.bitmap?.close(); return; }
    if (data.type === 'started') { running = true; paused = false; controls(true); status(extra.library ? 'Library' : `Running · ${logical.join(' × ')}`); canvas.focus(); }
    if (data.type === 'quit') { void chosen(data.save); }
    if (data.type === 'present') { display.transferFromImageBitmap(data.bitmap); session.postMessage({ type: 'present-ready' }); }
    if (data.type === 'input-ready') { busy = false; }
    if (data.type === 'saved' || data.type === 'loaded') { operation = false; busy = false; status(data.type === 'saved' ? 'Saved in this browser.' : 'Save loaded.'); }
    if (data.type === 'inspection') window.crysirrorInspection = data.state;
    if (data.type === 'error') { operation = false; busy = false; status(data.message, true); if (data.fatal) { running = false; controls(false); void stop(); } }
  };
  status('Starting the shared engine…');
  worker.postMessage({ type: 'boot', audio: output.port, project, names, bytes, saveKey, ...extra }, [output.port, ...bytes.map(b => b.buffer)]);
}
// The library: built-in demos plus projects opened in this session, shown
// by the shared launcher (crysirror-library) inside the canvas.
const imports = [];
const LIBRARY_SIZE = [1280, 720];
async function fetchBytes(path) {
  const r = await fetch(path); if (!r.ok) throw new Error(`Could not load ${path}`);
  return new Uint8Array(await r.arrayBuffer());
}
const demos = [
  { id: 'demo-2d', name: '2D demo', detail: 'Images, input waits and a generated tone', async start() {
    const project = new TextDecoder().decode(await fetchBytes('./demo/project.json'));
    const names = ['assets/card.png', 'assets/tone.wav'];
    await boot(project, names, await Promise.all(names.map(n => fetchBytes(`./demo/${n}`))), 'crysirror-demo-v1');
  } },
  { id: 'demo-3d', name: '3D demo', detail: 'PBR cubes through the shared renderer', async start() {
    const project = new TextDecoder().decode(await fetchBytes('./demo/pbr.json'));
    await boot(project, ['assets/card.png'], [await fetchBytes('./demo/assets/card.png')], 'crysirror-pbr-v1');
  } },
];
// The owner font lives in this site's IndexedDB; none is bundled.
function fontStore(mode, use) {
  return new Promise((ok, fail) => {
    const open = indexedDB.open('crysirror-web-fonts', 1);
    open.onupgradeneeded = () => open.result.createObjectStore('fonts');
    open.onerror = () => fail(open.error);
    open.onsuccess = () => {
      const tx = open.result.transaction('fonts', mode); const req = use(tx.objectStore('fonts'));
      tx.oncomplete = () => ok(req.result); tx.onerror = () => fail(tx.error);
    };
  });
}
const loadFont = () => fontStore('readonly', s => s.get('default'));
async function showLibrary() {
  try {
    const font = await loadFont();
    if (!font) {
      await stop();
      $('placeholder-title').textContent = 'Choose a font to show the library';
      $('placeholder-text').textContent = 'Use Font in the top bar (TTF/OTF/TTC with the scripts your games use). It stays in this browser.';
      status('A font is needed for the library.');
      return;
    }
    const all = [...demos, ...imports];
    const entries = all.map(e => ({ id: e.id, name: e.name, detail: e.detail }));
    const covers = all.map(e => e.cover ?? null);
    await boot('', [], [], 'crysirror-library-v1', { library: { entries, covers, size: LIBRARY_SIZE }, font: new Uint8Array(font) });
  } catch (e) { status(String(e), true); }
}
async function chosen(save) {
  let id = null;
  try { id = JSON.parse(save ?? 'null')?.game ?? null; } catch {}
  const entry = [...demos, ...imports].find(e => e.id === id);
  if (!entry) { await showLibrary(); return; }
  try { await entry.start(); } catch (e) { status(String(e), true); }
}
$('home').onclick = () => showLibrary();
$('font').onchange = async event => {
  const file = event.target.files[0]; if (!file) return;
  if (file.size > 32 * 1024 * 1024) { status('Fonts are limited to 32 MiB.', true); return; }
  const bytes = await file.arrayBuffer();
  await fontStore('readwrite', s => s.put(bytes, 'default'));
  status(`Font ${file.name} saved in this browser.`);
  await showLibrary();
};
// Import a project folder: `entries` are {file, path} with paths relative to
// the chosen folder's parent ("folder/project.json").
async function importFolder(entries) {
  try {
    if (!entries.length) return;
    const files = entries.map(e => e.file);
    if (files.length > 65536 || files.reduce((n, f) => n + f.size, 0) > 256 * 1024 * 1024 || files.some(f => f.size > 64 * 1024 * 1024)) throw new Error('Import limits: 256 MiB total, 64 MiB per file, 65536 files.');
    const root = entries[0].path.split('/')[0] + '/';
    const names = entries.map(e => e.path.slice(root.length));
    const index = names.indexOf('project.json'); if (index < 0) throw new Error('Choose a CrySirror project folder with project.json at its root.');
    const project = await files[index].text(); const parsed = JSON.parse(project);
    const identity = new TextEncoder().encode(JSON.stringify([parsed.name, parsed.clients.map(c => [c.adapter, c.game, c.name])]));
    const digest = await crypto.subtle.digest('SHA-256', identity);
    const saveKey = 'project-' + [...new Uint8Array(digest)].map(b => b.toString(16).padStart(2, '0')).join('');
    const coverIndex = names.findIndex(n => ['cover.png', 'cover.jpg', 'cover.jpeg', 'thumbnail.png', 'icon.png'].includes(n.toLowerCase()));
    const entry = {
      id: saveKey, name: parsed.name || root.slice(0, -1), detail: root.slice(0, -1),
      cover: coverIndex >= 0 ? new Uint8Array(await files[coverIndex].arrayBuffer()) : null,
      async start() {
        const bytes = await Promise.all(files.map(f => f.arrayBuffer().then(b => new Uint8Array(b))));
        await boot(project, names, bytes, saveKey);
      },
    };
    const existing = imports.findIndex(e => e.id === entry.id);
    if (existing >= 0) imports[existing] = entry; else imports.push(entry);
    await entry.start();
  } catch (e) { status(String(e), true); }
}
$('folder').onchange = event => importFolder([...event.target.files].map(file => ({ file, path: file.webkitRelativePath })));
// Drag a project folder onto the "Open project" card.
async function walk(entry, out) {
  if (entry.isFile) {
    out.push({ file: await new Promise((ok, fail) => entry.file(ok, fail)), path: entry.fullPath.replace(/^\//, '') });
  } else if (entry.isDirectory) {
    const reader = entry.createReader();
    for (;;) {
      const batch = await new Promise((ok, fail) => reader.readEntries(ok, fail));
      if (!batch.length) break;
      for (const child of batch) await walk(child, out);
    }
  }
}
const drop = $('drop');
document.addEventListener('dragover', e => { e.preventDefault(); document.body.classList.add('dragging'); });
document.addEventListener('dragleave', e => { if (!e.relatedTarget) document.body.classList.remove('dragging'); });
document.addEventListener('drop', async e => {
  e.preventDefault(); document.body.classList.remove('dragging');
  const roots = [...e.dataTransfer.items].map(i => i.webkitGetAsEntry?.()).filter(Boolean);
  if (roots.length !== 1 || !roots[0].isDirectory) { status('Drop one project folder.', true); return; }
  const entries = []; await walk(roots[0], entries); await importFolder(entries);
});
drop.onkeydown = e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); $('folder').click(); } };
$('fullscreen').onclick = () => { $('viewport').requestFullscreen?.().then(() => $('game').focus()).catch(e => status(String(e), true)); };
$('pause').onclick = async () => {
  paused = !paused; down = pressed = 0; $('pause').textContent = paused ? 'Resume' : 'Pause';
  worker?.postMessage({ type: 'pause', paused });
  if (paused) await audioContext?.suspend(); else { await audioContext?.resume(); }
};
for (const type of ['save', 'load']) $(type).onclick = () => {
  if (!running || operation) return;
  // Commands wait behind the last admitted frame. No Rust mutable borrow is
  // held across a second frame while IndexedDB is committing.
  operation = true; worker.postMessage({ type });
};
function setButton(bit, active) { if (active) { if (!(down & bit)) pressed |= bit; down |= bit; } else down &= ~bit; }
function bindInput(canvas) {
  canvas.onkeydown = e => { const bit = keyBits.get(e.code); if (bit) { e.preventDefault(); setButton(bit, true); } };
  canvas.onkeyup = e => { const bit = keyBits.get(e.code); if (bit) { e.preventDefault(); setButton(bit, false); } };
  canvas.oncontextmenu = e => e.preventDefault();
  canvas.onpointermove = e => {
    const rect = canvas.getBoundingClientRect(), scale = Math.min(rect.width / logical[0], rect.height / logical[1]);
    pointer = [Math.floor((e.clientX - rect.left - (rect.width - logical[0] * scale) / 2) / scale), Math.floor((e.clientY - rect.top - (rect.height - logical[1] * scale) / 2) / scale)];
  };
  canvas.onpointerdown = e => { canvas.focus(); canvas.onpointermove(e); canvas.setPointerCapture(e.pointerId); setButton(e.button === 2 ? 1 << 17 : 1 << 16, true); };
  canvas.onpointerup = e => setButton(e.button === 2 ? 1 << 17 : 1 << 16, false);
  canvas.onpointercancel = canvas.onblur = () => { down = pressed = 0; };
  canvas.onpointerleave = () => { pointer = [-1, -1]; };
}
bindInput($('game'));
window.addEventListener('blur', () => { down = pressed = 0; });
document.addEventListener('visibilitychange', () => {
  down = pressed = 0;
  worker?.postMessage({ type: 'pause', paused: paused || document.hidden });
  if (paused || document.hidden) audioContext?.suspend(); else audioContext?.resume();
});
// A read-only inspection message for local validation and host diagnostics.
window.crysirrorInspect = () => worker?.postMessage({ type: 'inspect' });
function frame() {
  requestAnimationFrame(frame);
  if (!running || paused || busy || operation || document.hidden) return;
  busy = true;
  worker.postMessage({ type: 'input', down, pressed, x: pointer[0], y: pointer[1], width: logical[0], height: logical[1] });
  pressed = 0;
}
requestAnimationFrame(frame);

void showLibrary();
