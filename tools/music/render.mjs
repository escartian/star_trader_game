// Exports a Strudel source (assets/source/music/*.strudel.js) to a seamless
// looping Ogg Vorbis file for the game. See README.md in this folder.
//
//   node tools/music/render.mjs [source.strudel.js] [output.ogg] [--bars N]
//
// Strudel renders offline in headless Chrome (OfflineAudioContext, faster than
// real time, no audio device). Two extra bars are rendered past the loop so the
// reverb and delay tails can be folded back onto the start: the loop point then
// has no click and no dry gap. The result is peak-normalized to -1 dBFS and
// encoded with ffmpeg.
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
const tailBars = 2;
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
window.render = async (code, cps, loopBars, tailBars, sampleRate) => {
  await initStrudel();
  // The tail bars only let notes from the loop ring out: no new onsets.
  const pattern = (await evaluate(code, false))
    .filterHaps((hap) => hap.whole.begin.valueOf() < loopBars);
  await renderPatternAudio(pattern, cps, 0, loopBars + tailBars, sampleRate, 256, false, 'render');
  const bytes = new Uint8Array(await window.renderedWav.arrayBuffer());
  let binary = '';
  for (let i = 0; i < bytes.length; i += 0x8000) binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(binary);
};
window.ready = true;
</script>`;

const dist = join(here, 'node_modules/@strudel/web/dist');
const server = createServer((request, response) => {
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
let wav;
try {
  const tab = await browser.newPage();
  tab.on('pageerror', (error) => console.error('page error:', error.message));
  await tab.goto(`http://127.0.0.1:${server.address().port}/`);
  await tab.waitForFunction('window.ready === true');
  console.log(`Rendering ${loopBars}+${tailBars} bars at ${(cps * 240).toFixed(1)} BPM…`);
  const base64 = await tab.evaluate((...a) => window.render(...a), code, cps, loopBars, tailBars, sampleRate);
  wav = Buffer.from(base64, 'base64');
} finally {
  await browser.close();
  server.close();
}

// 16-bit stereo PCM from Strudel's WAV writer: fold the tail onto the head.
const channels = wav.readUInt16LE(22);
const dataStart = wav.indexOf('data', 36) + 8;
const frames = (wav.length - dataStart) / 2 / channels;
const loopFrames = Math.round((loopBars / cps) * sampleRate);
if (channels !== 2 || frames < loopFrames) throw new Error('Unexpected render format');
const mix = new Float32Array(loopFrames * 2);
for (let frame = 0; frame < frames; frame += 1) {
  const target = (frame % loopFrames) * 2;
  for (let channel = 0; channel < 2; channel += 1) {
    mix[target + channel] += wav.readInt16LE(dataStart + (frame * 2 + channel) * 2) / 32768;
  }
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
