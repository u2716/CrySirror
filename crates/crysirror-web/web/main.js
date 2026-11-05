// SPDX-License-Identifier: MPL-2.0
const $ = id => document.getElementById(id);
const status = (text, error = false) => { $('status').textContent = text; $('status').className = error ? 'error' : ''; };
let worker, audioContext, output, busy = false, running = false, paused = false, operation = false;
let down = 0, pressed = 0, pointer = [-1, -1], logical = [640, 360];
const keyBits = new Map([['Enter', 1], ['Space', 2], ['ArrowLeft', 4], ['ArrowRight', 8], ['ArrowUp', 16], ['ArrowDown', 32], ['Tab', 64], ['KeyW', 128], ['KeyA', 256], ['KeyS', 512], ['KeyD', 1024], ['ShiftLeft', 2048], ['ControlLeft', 4096], ['Escape', 1 << 19]]);
function controls(enabled) { for (const id of ['pause', 'save', 'load']) $(id).disabled = !enabled; }
async function stop() {
  running = false; busy = false; operation = false; worker?.terminate(); worker = undefined;
  output?.disconnect(); if (audioContext) await audioContext.close(); audioContext = undefined; output = undefined;
  down = pressed = 0; paused = false; controls(false); $('pause').textContent = 'Pause';
}
async function boot(project, names, bytes, saveKey) {
  await stop();
  if (!navigator.gpu || typeof OffscreenCanvas !== 'function') throw new Error('WebGPU and OffscreenCanvas are required. Open this host through localhost or HTTPS.');
  logical = JSON.parse(project).canvas;
  audioContext = new AudioContext({ sampleRate: 48000 });
  const resume = audioContext.resume();
  await audioContext.audioWorklet.addModule('./audio.js'); await resume;
  output = new AudioWorkletNode(audioContext, 'crysirror-output', { outputChannelCount: [2] }); output.connect(audioContext.destination);
  const old = $('game'); const canvas = old.cloneNode(); old.replaceWith(canvas); canvas.width = logical[0]; canvas.height = logical[1]; bindInput(canvas);
  $('viewport').style.width = `${logical[0]}px`; $('viewport').style.height = `${logical[1]}px`;
  const display = canvas.getContext('bitmaprenderer');
  if (!display) throw new Error('This browser does not support ImageBitmap presentation.');
  const session = new Worker('./worker.js', { type: 'module' });
  worker = session;
  session.onerror = event => { if (worker !== session) return; status(`Browser engine error: ${event.message}`, true); running = false; controls(false); };
  session.onmessage = ({ data }) => {
    if (worker !== session) { data.bitmap?.close(); return; }
    if (data.type === 'started') { running = true; paused = false; controls(true); status(`Running · ${logical.join(' × ')} · Click the canvas or press Enter`); canvas.focus(); }
    if (data.type === 'present') { display.transferFromImageBitmap(data.bitmap); session.postMessage({ type: 'present-ready' }); }
    if (data.type === 'input-ready') { busy = false; }
    if (data.type === 'saved' || data.type === 'loaded') { operation = false; busy = false; status(data.type === 'saved' ? 'Saved in this browser.' : 'Save loaded.'); }
    if (data.type === 'inspection') window.crysirrorInspection = data.state;
    if (data.type === 'error') { operation = false; busy = false; status(data.message, true); if (data.fatal) { running = false; controls(false); void stop(); } }
  };
  status('Starting the shared engine…');
  worker.postMessage({ type: 'boot', audio: output.port, project, names, bytes, saveKey }, [output.port, ...bytes.map(b => b.buffer)]);
}
$('demo').onclick = async () => {
  try {
    const project = await fetch('./demo/project.json').then(r => { if (!r.ok) throw new Error('Could not load the demo project'); return r.text(); });
    const names = ['assets/card.png', 'assets/tone.wav'];
    const bytes = await Promise.all(names.map(n => fetch(`./demo/${n}`).then(r => { if (!r.ok) throw new Error(`Could not load: ${n}`); return r.arrayBuffer(); }).then(b => new Uint8Array(b))));
    await boot(project, names, bytes, 'crysirror-demo-v1');
  } catch (e) { status(String(e), true); }
};
$('demo3d').onclick = async () => {
  try {
    const response = await fetch('./demo/pbr.json');
    if (!response.ok) throw new Error('Could not load the PBR demo');
    const project = await response.text();
    const bytes = await fetch('./demo/assets/card.png').then(r => r.arrayBuffer()).then(b => new Uint8Array(b));
    await boot(project, ['assets/card.png'], [bytes], 'crysirror-pbr-v1');
  } catch (e) { status(String(e), true); }
};
$('folder').onchange = async event => {
  try {
    const files = [...event.target.files]; if (!files.length) return;
    if (files.length > 65536 || files.reduce((n, f) => n + f.size, 0) > 256 * 1024 * 1024 || files.some(f => f.size > 64 * 1024 * 1024)) throw new Error('Import limits: 256 MiB total, 64 MiB per file, 65536 files.');
    const root = files[0].webkitRelativePath.split('/')[0] + '/';
    const names = files.map(f => f.webkitRelativePath.slice(root.length));
    const index = names.indexOf('project.json'); if (index < 0) throw new Error('Choose a CrySirror project folder with project.json at its root.');
    const project = await files[index].text(); const parsed = JSON.parse(project);
    const identity = new TextEncoder().encode(JSON.stringify([parsed.name, parsed.clients.map(c => [c.adapter, c.game, c.name])]));
    const digest = await crypto.subtle.digest('SHA-256', identity);
    const saveKey = 'project-' + [...new Uint8Array(digest)].map(b => b.toString(16).padStart(2, '0')).join('');
    const bytes = await Promise.all(files.map(f => f.arrayBuffer().then(b => new Uint8Array(b))));
    await boot(project, names, bytes, saveKey);
  } catch (e) { status(String(e), true); }
};
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
