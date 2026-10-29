// The core message codec for TypeScript guests, following
// wit/wire-format.md: little-endian numbers, u16-prefixed strings,
// u32-prefixed blobs, and "rest" fields that take every remaining byte.

import { FIELDS, type Messages } from "./messages.ts";

/** A field type of wit/wire-format.md that has a fixed TypeScript form. */
export type FieldType =
  | "u8" | "u16" | "u32" | "i32" | "f32" | "bool"
  | "str" | "str-rest" | "blob" | "rest";

const utf8 = new TextEncoder();
const fromUtf8 = new TextDecoder("utf-8", { fatal: true });

/** The payload bytes of `message` on `topic`. */
export function encode<T extends keyof Messages>(topic: T, message: Messages[T]): Uint8Array {
  const parts: Uint8Array[] = [];
  for (const [name, type] of FIELDS[topic]) {
    parts.push(encodeField(type, (message as Record<string, unknown>)[name]));
  }
  const bytes = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
  let offset = 0;
  for (const part of parts) {
    bytes.set(part, offset);
    offset += part.length;
  }
  return bytes;
}

/** The message a `topic` payload carries. Throws on a short or long payload. */
export function decode<T extends keyof Messages>(topic: T, bytes: Uint8Array): Messages[T] {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const message: Record<string, unknown> = {};
  let offset = 0;
  const take = (length: number): Uint8Array => {
    if (offset + length > bytes.length) {
      throw new Error(`${topic}: payload ended before an expected field`);
    }
    offset += length;
    return bytes.subarray(offset - length, offset);
  };
  for (const [name, type] of FIELDS[topic]) {
    switch (type) {
      case "u8": message[name] = take(1)[0]; break;
      case "bool": message[name] = take(1)[0] !== 0; break;
      case "u16": take(2); message[name] = view.getUint16(offset - 2, true); break;
      case "u32": take(4); message[name] = view.getUint32(offset - 4, true); break;
      case "i32": take(4); message[name] = view.getInt32(offset - 4, true); break;
      case "f32": take(4); message[name] = view.getFloat32(offset - 4, true); break;
      case "str": message[name] = fromUtf8.decode(take(take(2).length && view.getUint16(offset - 2, true))); break;
      case "blob": message[name] = take(take(4).length && view.getUint32(offset - 4, true)).slice(); break;
      case "str-rest": message[name] = fromUtf8.decode(take(bytes.length - offset)); break;
      case "rest": message[name] = take(bytes.length - offset).slice(); break;
    }
  }
  if (offset !== bytes.length) {
    throw new Error(`${topic}: payload has bytes left after decoding`);
  }
  return message as unknown as Messages[T];
}

function encodeField(type: FieldType, value: unknown): Uint8Array {
  const fixed = (length: number, write: (view: DataView) => void): Uint8Array => {
    const bytes = new Uint8Array(length);
    write(new DataView(bytes.buffer));
    return bytes;
  };
  switch (type) {
    case "u8": return Uint8Array.of(Number(value));
    case "bool": return Uint8Array.of(value ? 1 : 0);
    case "u16": return fixed(2, (view) => view.setUint16(0, Number(value), true));
    case "u32": return fixed(4, (view) => view.setUint32(0, Number(value), true));
    case "i32": return fixed(4, (view) => view.setInt32(0, Number(value), true));
    case "f32": return fixed(4, (view) => view.setFloat32(0, Number(value), true));
    case "str": {
      const text = utf8.encode(String(value));
      return concat(fixed(2, (view) => view.setUint16(0, text.length, true)), text);
    }
    case "blob": {
      const bytes = value as Uint8Array;
      return concat(fixed(4, (view) => view.setUint32(0, bytes.length, true)), bytes);
    }
    case "str-rest": return utf8.encode(String(value));
    case "rest": return value as Uint8Array;
  }
}

function concat(head: Uint8Array, tail: Uint8Array): Uint8Array {
  const bytes = new Uint8Array(head.length + tail.length);
  bytes.set(head);
  bytes.set(tail, head.length);
  return bytes;
}
