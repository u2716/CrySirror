// SPDX-License-Identifier: MPL-2.0
// Bounded output-only ring. The engine owns decoding, channel state and mixing.
class CrySirrorOutput extends AudioWorkletProcessor {
  constructor() {
    super(); this.ring = new Float32Array(48000 * 2); this.read = 0; this.count = 0; this.phase = 0; this.frames = 0; this.nonzero = 0; this.peak = 0; this.reported = 0;
    this.port.onmessage = ({ data }) => {
      if (data?.type === 'clear') { this.count = 0; this.read = 0; this.phase = 0; return; }
      if (!(data instanceof Float32Array) || data.length % 2) return;
      // Keep at most a quarter second queued. Slow/background presentation
      // must not produce an ever-growing delayed soundtrack.
      const frames = Math.min(data.length / 2, 12000);
      const skip = Math.max(0, this.count + frames - 12000);
      this.read = (this.read + skip) % 48000; this.count -= skip;
      for (let i = 0; i < frames; i++) {
        const at = (this.read + this.count + i) % 48000;
        this.ring[at * 2] = data[i * 2]; this.ring[at * 2 + 1] = data[i * 2 + 1];
      }
      this.count += frames;
    };
  }
  process(_inputs, outputs) {
    const [left, right] = outputs[0];
    const ratio = 48000 / sampleRate;
    for (let i = 0; i < left.length; i++) {
      if (!this.count) { left[i] = 0; if (right) right[i] = 0; continue; }
      const at = this.read * 2;
      left[i] = Math.max(-1, Math.min(1, this.ring[at]));
      if (right) right[i] = Math.max(-1, Math.min(1, this.ring[at + 1]));
      const peak = Math.max(Math.abs(left[i]), right ? Math.abs(right[i]) : 0);
      this.peak = Math.max(this.peak, peak); if (peak > 0) this.nonzero++;
      this.phase += ratio;
      const n = Math.min(Math.floor(this.phase), this.count);
      this.phase -= n; this.read = (this.read + n) % 48000; this.count -= n;
      if (!this.count) this.phase = 0;
    }
    this.frames += left.length;
    if (this.frames - this.reported >= sampleRate) {
      this.reported = this.frames;
      this.port.postMessage({ frames: this.frames, nonzeroFrames: this.nonzero, peak: this.peak, sampleRate });
    }
    return true;
  }
}
registerProcessor('crysirror-output', CrySirrorOutput);
