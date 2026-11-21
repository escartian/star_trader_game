// Exports a Strudel source (assets/source/music/*.strudel.js) to a seamless
// looping Ogg Vorbis file for the game. See README.md in this folder.
//
//   node tools/music/render.mjs [source.strudel.js] [output.ogg] [--bars N]
//
// Strudel renders offline in headless Chrome (OfflineAudioContext, faster than
// real time, no audio device). It renders one bar at a time, each at its own
// tempo: from the source's `window.tempoMap` (BPM for every bar), or else N
// bars at its `setcps` tempo. Each bar's notes ring out for `TAIL_SECONDS`, and
// the bars are added together at their start times. What rings past the loop's
// end is folded back onto the start, so the loop point has no click and no dry
// gap. The result is peak-normalized to -1 dBFS and encoded with ffmpeg.
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdtempSync, rmSync, existsSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import puppeteer from 'puppeteer-core';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '../..');
const args = process.argv.slice(2);
const flag = (name, fallback) => {
  const index = args.indexOf(name);
  if (index < 0) return fallback;
  const [, value] = args.splice(index, 2);
  return value;
};
const loopBars = Number(flag('--bars', '32'));
const source = resolve(args[0] ?? join(root, 'assets/source/music/main-theme.strudel.js'));
const output = resolve(args[1] ?? join(root, 'assets/music/main-theme.ogg'));
/// Seconds each bar's reverb and delay are allowed to ring after it.
const TAIL_SECONDS = 9;
const sampleRate = 48_000;
const chrome = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';

const code = readFileSync(source, 'utf8');
// Tempo in cycles per second, from the source's own `setcps(...)` line.
const tempo = code.match(/^setcps\(([\d\s./*+-]+)\)/m);
if (!tempo) throw new Error(`${source} needs a plain arithmetic setcps(...) line`);
const cps = Function(`return (${tempo[1]})`)();
if (!(cps > 0)) throw new Error(`Bad tempo ${tempo[1]}`);
if (!existsSync(chrome)) throw new Error(`Chrome not found at ${chrome}; set CHROME=/path/to/chrome`);

const page = `<!doctype html><meta charset="utf-8"><script type="module">
import { initStrudel, evaluate, renderPatternAudio } from '/strudel/index.mjs';
// renderPatternAudio "downloads" its WAV through an object URL; keep the blob.
const createObjectURL = URL.createObjectURL;
URL.createObjectURL = (blob) => { window.renderedWav = blob; return createObjectURL(blob); };
// Strudel builds each reverb's impulse response asynchronously (in a nested
// offline render). An offline render would start before it arrives and play
// no reverb at all, so renders wait until every reverb has its response.
const convolvers = new WeakMap();
const createConvolver = BaseAudioContext.prototype.createConvolver;
BaseAudioContext.prototype.createConvolver = function () {
  const node = createConvolver.call(this);
  if (!convolvers.has(this)) convolvers.set(this, []);
  convolvers.get(this).push(node);
  return node;
};
const startRendering = OfflineAudioContext.prototype.startRendering;
OfflineAudioContext.prototype.startRendering = async function () {
  const waiting = convolvers.get(this) ?? [];
  for (let tries = 0; tries < 500 && waiting.some((node) => !node.buffer); tries += 1) {
    await new Promise((done) => setTimeout(done, 10));
  }
  if (waiting.some((node) => !node.buffer)) console.error('a reverb impulse response never arrived');
  return startRendering.call(this);
};
// Evaluates the piece once; returns its tempo map (BPM per bar) if it has one.
window.load = async (code) => {
  await initStrudel();
  window.piece = await evaluate(code, false);
  return window.tempoMap ?? null;
};
// Renders the notes that start in \`bar\`, at \`cps\`, ringing \`tailCycles\` longer.
window.renderBar = async (bar, cps, tailCycles, sampleRate) => {
  const pattern = window.piece.filterHaps((hap) => {
    const begin = hap.whole.begin.valueOf();
    return begin >= bar && begin < bar + 1;
  });
  await renderPatternAudio(pattern, cps, bar, bar + 1 + tailCycles, sampleRate, 256, false, 'render');
  const bytes = new Uint8Array(await window.renderedWav.arrayBuffer());
  let binary = '';
  for (let i = 0; i < bytes.length; i += 0x8000) binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(binary);
};
window.ready = true;
</script>`;

const dist = join(here, 'node_modules/@strudel/web/dist');
const server = createServer((request, response) => {
  if (request.url === '/favicon.ico') {
    response.writeHead(204).end();
    return;
  }
  if (request.url === '/') {
    response.writeHead(200, { 'content-type': 'text/html' }).end(page);
    return;
  }
  const path = request.url.startsWith('/strudel/') ? join(dist, request.url.slice(9).split('?')[0]) : null;
  if (!path || !path.startsWith(dist) || !existsSync(path)) {
    response.writeHead(404).end();
    return;
  }
  response.writeHead(200, { 'content-type': 'text/javascript' }).end(readFileSync(path));
});
await new Promise((done) => server.listen(0, '127.0.0.1', done));
const browser = await puppeteer.launch({
  executablePath: chrome,
  headless: true,
  args: ['--autoplay-policy=no-user-gesture-required'],
});
let mix;
let loopFrames;
try {
  const tab = await browser.newPage();
  tab.on('pageerror', (error) => console.error('page error:', error.message));
  // Strudel reports unknown sounds and bad values through the console.
  tab.on('console', (message) => {
    const loud = ['error', 'warn', 'warning'].includes(message.type());
    if (loud || process.env.DEBUG_BARS) console.warn(`strudel ${message.type()}:`, message.text());
  });
  // Strudel keeps audio nodes from one offline render to the next, which
  // breaks later renders in the same page, so each bar gets a fresh page.
  const fresh = async () => {
    await tab.goto(`http://127.0.0.1:${server.address().port}/`);
    await tab.waitForFunction('window.ready === true');
    return tab.evaluate((piece) => window.load(piece), code);
  };
  const tempoMap = await fresh();
  const bpms = tempoMap ?? Array(loopBars).fill(cps * 240);
  if (!bpms.length || bpms.some((bpm) => !(bpm > 0))) throw new Error('Bad tempoMap');
  const starts = bpms.map((_, bar) => bpms.slice(0, bar).reduce((sum, bpm) => sum + 240 / bpm, 0));
  const seconds = starts[bpms.length - 1] + 240 / bpms[bpms.length - 1];
  loopFrames = Math.round(seconds * sampleRate);
  mix = new Float32Array(loopFrames * 2);
  const range = `${Math.min(...bpms)}-${Math.max(...bpms)}`;
  console.log(`Rendering ${bpms.length} bars at ${range} BPM, ${seconds.toFixed(1)} s…`);
  for (let bar = 0; bar < bpms.length; bar += 1) {
    if (bar > 0) await fresh();
    const barCps = bpms[bar] / 240;
    const base64 = await tab.evaluate(
      (...a) => window.renderBar(...a), bar, barCps, TAIL_SECONDS * barCps, sampleRate,
    );
    const peak = addBar(mix, Buffer.from(base64, 'base64'), Math.round(starts[bar] * sampleRate), loopFrames);
    if (process.env.DEBUG_BARS) console.log(`  bar ${bar + 1}: peak ${peak.toFixed(3)}`);
    if (peak < 1e-3) console.warn(`  bar ${bar + 1} rendered silent`);
    if ((bar + 1) % 8 === 0) console.log(`  ${bar + 1}/${bpms.length} bars`);
  }
} finally {
  await browser.close();
  server.close();
}

/** Adds a bar's 16-bit stereo WAV into `mix` from frame `offset`, wrapping
 *  past the loop's end onto its start. Returns the bar's peak level. */
function addBar(mix, wav, offset, loopFrames) {
  const channels = wav.readUInt16LE(22);
  const dataStart = wav.indexOf('data', 36) + 8;
  if (channels !== 2) throw new Error('Unexpected render format');
  const frames = (wav.length - dataStart) / 4;
  let peak = 0;
  for (let frame = 0; frame < frames; frame += 1) {
    const target = ((offset + frame) % loopFrames) * 2;
    const left = wav.readInt16LE(dataStart + frame * 4) / 32768;
    const right = wav.readInt16LE(dataStart + frame * 4 + 2) / 32768;
    mix[target] += left;
    mix[target + 1] += right;
    peak = Math.max(peak, Math.abs(left), Math.abs(right));
  }
  return peak;
}

const peak = mix.reduce((max, sample) => Math.max(max, Math.abs(sample)), 0);
if (peak === 0) throw new Error('Render is silent');
const scale = 10 ** (-1 / 20) / peak;
const pcm = Buffer.alloc(44 + mix.length * 2);
pcm.write('RIFF', 0);
pcm.writeUInt32LE(36 + mix.length * 2, 4);
pcm.write('WAVEfmt ', 8);
pcm.writeUInt32LE(16, 16);
pcm.writeUInt16LE(1, 20);
pcm.writeUInt16LE(2, 22);
pcm.writeUInt32LE(sampleRate, 24);
pcm.writeUInt32LE(sampleRate * 4, 28);
pcm.writeUInt16LE(4, 32);
pcm.writeUInt16LE(16, 34);
pcm.write('data', 36);
pcm.writeUInt32LE(mix.length * 2, 40);
mix.forEach((sample, index) => {
  pcm.writeInt16LE(Math.max(-32768, Math.min(32767, Math.round(sample * scale * 32767))), 44 + index * 2);
});
const scratch = mkdtempSync(join(tmpdir(), 'star-trader-music-'));
try {
  const loop = join(scratch, 'loop.wav');
  writeFileSync(loop, pcm);
  // Vorbis keeps the loop sample-exact (MP3 pads the start and end). Prefer
  // libvorbis; ffmpeg's built-in encoder is weaker, so it gets a high bitrate.
  const encoders = execFileSync('ffmpeg', ['-hide_banner', '-encoders']).toString();
  const codec = /\blibvorbis\b/.test(encoders)
    ? ['-c:a', 'libvorbis', '-q:a', '5']
    : ['-c:a', 'vorbis', '-strict', 'experimental', '-b:a', '192k'];
  if (codec[1] === 'vorbis') console.warn('ffmpeg has no libvorbis; using its built-in Vorbis encoder');
  execFileSync('ffmpeg', ['-loglevel', 'error', '-y', '-i', loop, ...codec, output]);
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
console.log(`Wrote ${output}: ${(loopFrames / sampleRate).toFixed(1)} s loop, peak gain ${(20 * Math.log10(scale)).toFixed(1)} dB`);
