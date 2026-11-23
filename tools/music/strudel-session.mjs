// A headless Chrome session that evaluates a Strudel source and renders it
// offline, one bar at a time (OfflineAudioContext: faster than real time, no
// audio device). Used by render.mjs.
//
// Sampled instruments come from Strudel's General MIDI soundfonts
// (@strudel/soundfonts). Their sample data is served from local copies in
// assets/source/music/soundfonts/, so a render never downloads anything; a
// sound whose font file is missing there fails to load and is reported.
import { existsSync, readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import puppeteer from 'puppeteer-core';

const here = dirname(fileURLToPath(import.meta.url));
const chrome = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';

const fonts = join(here, '../../assets/source/music/soundfonts');

// The soundfont loader imports @strudel/core and @strudel/webaudio; both map
// to the one bundled Strudel so its sounds register with the same engine.
const page = `<!doctype html><meta charset="utf-8">
<script type="importmap">
{ "imports": { "@strudel/core": "/strudel/index.mjs", "@strudel/webaudio": "/strudel/index.mjs" } }
</script>
<script type="module">
import { initStrudel, evaluate, renderPatternAudio } from '/strudel/index.mjs';
import { registerSoundfonts, setSoundfontUrl } from '/soundfonts/fontloader.mjs';
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
// Evaluates the piece. Returns what it declares: a tempo map (BPM per bar),
// or blocks (each with its tempo per bar and the blocks that may follow).
window.load = async (code) => {
  await initStrudel();
  setSoundfontUrl('/fonts');
  registerSoundfonts();
  window.piece = await evaluate(code, false);
  const blocks = window.blocks
    ? Object.fromEntries(Object.entries(window.blocks).map(([name, block]) =>
        [name, { from: block.from, to: block.to, bpm: block.bpm, next: block.next, family: block.family }]))
    : null;
  return { tempoMap: window.tempoMap ?? null, blocks, start: window.musicStart ?? null };
};
// Renders the notes that start in bar \`bar\` (of a block, or of the whole
// piece), at \`cps\`, ringing \`tailCycles\` longer. Returns a base64 WAV.
window.renderBar = async (bar, cps, tailCycles, sampleRate, block) => {
  const source = block ? window.blocks[block].pattern : window.piece;
  const pattern = source.filterHaps((hap) => {
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

/** Opens a session for Strudel `code`. `load()` evaluates it in a fresh page
 *  (Strudel keeps audio nodes from one offline render to the next, which
 *  breaks later renders in the same page, so call it before every bar);
 *  `renderBar(...)` returns a 16-bit stereo WAV buffer; `close()` ends it. */
export async function openSession(code) {
  if (!existsSync(chrome)) throw new Error(`Chrome not found at ${chrome}; set CHROME=/path/to/chrome`);
  const dist = join(here, 'node_modules/@strudel/web/dist');
  const soundfonts = join(here, 'node_modules/@strudel/soundfonts');
  // URL prefix -> directory; every file is JavaScript.
  const roots = [['/strudel/', dist], ['/soundfonts/', soundfonts], ['/fonts/', fonts]];
  const server = createServer((request, response) => {
    if (request.url === '/favicon.ico') {
      response.writeHead(204).end();
      return;
    }
    if (request.url === '/') {
      response.writeHead(200, { 'content-type': 'text/html' }).end(page);
      return;
    }
    const root = roots.find(([prefix]) => request.url.startsWith(prefix));
    const path = root ? join(root[1], request.url.slice(root[0].length).split('?')[0]) : null;
    if (!path || !path.startsWith(root[1]) || !existsSync(path)) {
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
  const tab = await browser.newPage();
  tab.on('pageerror', (error) => console.error('page error:', error.message));
  // Strudel reports unknown sounds and bad values through the console.
  tab.on('console', (message) => {
    const loud = ['error', 'warn', 'warning'].includes(message.type());
    if (loud || process.env.DEBUG_BARS) console.warn(`strudel ${message.type()}:`, message.text());
  });
  return {
    async load() {
      await tab.goto(`http://127.0.0.1:${server.address().port}/`);
      await tab.waitForFunction('window.ready === true');
      return tab.evaluate((piece) => window.load(piece), code);
    },
    async renderBar(bar, cps, tailCycles, sampleRate, block = null) {
      const base64 = await tab.evaluate(
        (...a) => window.renderBar(...a), bar, cps, tailCycles, sampleRate, block,
      );
      return Buffer.from(base64, 'base64');
    },
    async close() {
      await browser.close();
      server.close();
    },
  };
}
