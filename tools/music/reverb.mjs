// A shared hall reverb for exported music. Strudel's offline renderer drops
// its own reverb (`room` has no effect there), so the export renders each
// layer's reverb send separately and runs it through this convolution reverb.
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

/** Decay time (to -60 dB) in seconds for a Strudel room size: `size(8)` is a
 *  hall of about 3.5 s. */
export const decaySeconds = (size) => 0.45 * size;

/** A stereo impulse response: a short pre-delay, then independent decaying
 *  noise per channel (for width), darkened above about 5 kHz and thinned
 *  below about 180 Hz so the low end stays clean. Unit energy per channel. */
export function impulse(seconds, sampleRate) {
  const length = Math.round(seconds * 1.2 * sampleRate);
  const predelay = Math.round(0.02 * sampleRate);
  const channels = [new Float32Array(length), new Float32Array(length)];
  let seed = 0x5eed;
  const random = () => {
    seed = (seed * 1664525 + 1013904223) >>> 0;
    return seed / 2 ** 31 - 1;
  };
  const low = Math.exp((-2 * Math.PI * 5000) / sampleRate);
  const high = Math.exp((-2 * Math.PI * 180) / sampleRate);
  for (const channel of channels) {
    let darkened = 0;
    let rumble = 0;
    for (let i = predelay; i < length; i += 1) {
      const t = (i - predelay) / sampleRate;
      const envelope = Math.exp((-6.9 * t) / seconds);
      darkened = (1 - low) * random() + low * darkened;
      rumble = (1 - high) * darkened + high * rumble;
      channel[i] = (darkened - rumble) * envelope;
    }
    const energy = Math.sqrt(channel.reduce((sum, value) => sum + value * value, 0));
    for (let i = 0; i < length; i += 1) channel[i] /= energy;
  }
  return channels;
}

/** 32-bit float stereo WAV bytes for interleaved samples. */
export function floatWav(interleaved, sampleRate) {
  const data = Buffer.alloc(interleaved.length * 4);
  interleaved.forEach((value, index) => data.writeFloatLE(value, index * 4));
  const header = Buffer.alloc(44);
  header.write('RIFF', 0);
  header.writeUInt32LE(36 + data.length, 4);
  header.write('WAVEfmt ', 8);
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(3, 20);
  header.writeUInt16LE(2, 22);
  header.writeUInt32LE(sampleRate, 24);
  header.writeUInt32LE(sampleRate * 8, 28);
  header.writeUInt16LE(8, 32);
  header.writeUInt16LE(32, 34);
  header.write('data', 36);
  header.writeUInt32LE(data.length, 40);
  return Buffer.concat([header, data]);
}

/** Reverberates a looping stereo send (interleaved, `frames` long) with the
 *  impulse: convolves two copies back to back and keeps the second, so the
 *  tail of the loop's end rings into its start. Returns interleaved samples. */
export function reverbLoop(send, frames, sampleRate, seconds, scratch) {
  const twice = new Float32Array(frames * 4);
  twice.set(send, 0);
  twice.set(send, frames * 2);
  const [left, right] = impulse(seconds, sampleRate);
  const ir = new Float32Array(left.length * 2);
  left.forEach((value, index) => {
    ir[index * 2] = value;
    ir[index * 2 + 1] = right[index];
  });
  const input = join(scratch, 'send.wav');
  const response = join(scratch, 'ir.wav');
  const output = join(scratch, 'wet.raw');
  writeFileSync(input, floatWav(twice, sampleRate));
  writeFileSync(response, floatWav(ir, sampleRate));
  execFileSync('ffmpeg', [
    '-loglevel', 'error', '-y', '-i', input, '-i', response,
    '-lavfi', 'afir=dry=1:wet=1:irnorm=-1:gtype=-1', '-f', 'f32le', '-ac', '2', output,
  ]);
  const wet = readFileSync(output);
  const result = new Float32Array(frames * 2);
  for (let index = 0; index < frames * 2; index += 1) {
    const at = (frames * 2 + index) * 4;
    result[index] = at + 4 <= wet.length ? wet.readFloatLE(at) : 0;
  }
  return result;
}
