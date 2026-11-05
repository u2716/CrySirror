// SPDX-License-Identifier: MPL-2.0
import * as wasm from './pkg/crysirror_web.js';
const { BrowserPlayer } = wasm;
let player, canvas, audio, presenting = false, operation = false, paused = false, failed = false, previous = 0;
let audioStats = null, cpu = null;
let input = { down: 0, pressed: 0, x: -1, y: -1, width: 640, height: 360 };
// Lifecycle/storage commands serialize around any await. No second message
// can borrow the Rust player while an async save/load still owns it.
let commands = Promise.resolve();
self.onmessage = ({ data }) => { commands = commands.then(() => handle(data)); };
function animate(time) {
  self.requestAnimationFrame(animate);
  const delta = previous ? Math.max(0, Math.min(100, Math.round(time - previous))) : 0;
  previous = time;
  if (!player || paused || operation || failed) return;
  try {
    const rendered = player.frame(delta, input.down, input.pressed, input.x, input.y, input.width, input.height);
    // One transferable GPU bitmap may be in flight. The page acknowledges
    // consumption before this worker transfers another generation.
    if (rendered && !presenting) {
      const bitmap = canvas.transferToImageBitmap();
      presenting = true;
      self.postMessage({ type: 'present', bitmap }, [bitmap]);
    }
    input.pressed = 0;
    const pcm = player.takeAudio();
    if (pcm.length) audio?.postMessage(pcm, [pcm.buffer]);
  } catch (error) {
    failed = true;
    self.postMessage({ type: 'error', fatal: true, message: error?.message ?? String(error) });
  }
}
async function handle(data) {
  try {
    if (data.type === 'boot') {
      if (player) throw new Error('The worker already owns a running project');
      if (wasm.initCpuWorkers && !self.crossOriginIsolated) throw new Error('CPU workers require cross-origin isolation (COOP/COEP headers). Use the supplied local server.');
      await wasm.default();
      if (wasm.initCpuWorkers) {
        const count = Math.max(2, Math.min(4, (navigator.hardwareConcurrency || 4) - 1));
        cpu = JSON.parse(await Promise.race([
          wasm.initCpuWorkers(count),
          new Promise((_, reject) => setTimeout(() => reject(new Error('CPU worker initialization timed out')), 15000)),
        ]));
      }
      audio = data.audio;
      audio.onmessage = ({ data }) => { audioStats = data; };
      const [width, height] = JSON.parse(data.project).canvas;
      canvas = new OffscreenCanvas(width, height);
      player = await BrowserPlayer.create(canvas, data.project, data.names, data.bytes, data.saveKey);
      [input.width, input.height] = JSON.parse(data.project).canvas;
      self.requestAnimationFrame(animate);
      self.postMessage({ type: 'started', state: JSON.parse(player.inspect()) });
    } else if (data.type === 'input' && player) {
      input = { ...data, pressed: input.pressed | data.pressed };
      self.postMessage({ type: 'input-ready' });
    } else if (data.type === 'present-ready') {
      presenting = false;
    } else if (data.type === 'pause') {
      paused = data.paused; input.down = input.pressed = 0;
      if (paused) audio?.postMessage({ type: 'clear' });
    } else if (data.type === 'inspect' && player) {
      self.postMessage({ type: 'inspection', state: { ...JSON.parse(player.inspect()), audio: audioStats, cpu } });
    } else if (data.type === 'save' && player) {
      operation = true;
      await player.saveSnapshot();
      self.postMessage({ type: 'saved' });
    } else if (data.type === 'load' && player) {
      operation = true;
      await player.loadSnapshot();
      audio?.postMessage({ type: 'clear' });
      self.postMessage({ type: 'loaded' });
    }
  } catch (error) {
    self.postMessage({ type: 'error', fatal: data.type === 'boot', message: error?.message ?? String(error) });
  } finally {
    if (data.type === 'save' || data.type === 'load') operation = false;
  }
}
