// Every conformance vector of a topic messages.ts covers must encode to its
// exact bytes and decode back to its exact values (wit/vectors/README.md).

import { deepStrictEqual, strictEqual } from "node:assert";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";

import { FIELDS, type Messages } from "../src/messages.ts";
import { decode, encode, type FieldType } from "../src/wire.ts";

const vectors = readFileSync(join(import.meta.dirname, "../../../../wit/vectors/vectors.txt"), "utf8");

const hex = (bytes: Uint8Array): string => Buffer.from(bytes).toString("hex");
const unhex = (text: string): Uint8Array => new Uint8Array(Buffer.from(text, "hex"));

/** A vector's text value as the type its field takes. */
function value(type: FieldType, text: string): unknown {
  switch (type) {
    case "bool": return text === "1" || text === "true";
    case "str": case "str-rest": return text;
    case "blob": case "rest": return unhex(text);
    default: return Number(text);
  }
}

test("every vector of a covered topic round-trips exactly", () => {
  let checked = 0;
  for (const line of vectors.split(/\r?\n/)) {
    if (line === "" || line.startsWith("#")) continue;
    // Three fields on the first two spaces; a value may contain spaces, and
    // JavaScript's split limit would drop them instead of keeping the rest.
    const first = line.indexOf(" ");
    const second = line.indexOf(" ", first + 1);
    const topic = line.slice(0, first);
    const payload = second < 0 ? line.slice(first + 1) : line.slice(first + 1, second);
    const values = second < 0 ? "" : line.slice(second + 1);
    if (!(topic in FIELDS)) continue;
    const fields = FIELDS[topic as keyof Messages];
    const message: Record<string, unknown> = {};
    for (const pair of values.split(";").filter(Boolean)) {
      const [name, text] = pair.split("=", 2);
      const field = fields.find(([field]) => field === name);
      if (field) message[name] = value(field[1], text);
    }
    const bytes = encode(topic as keyof Messages, message as never);
    strictEqual(hex(bytes), payload === "-" ? "" : payload, line);
    const decoded = decode(topic as keyof Messages, bytes) as Record<string, unknown>;
    for (const [name, type] of fields) {
      if (type === "f32") strictEqual(Math.fround(decoded[name] as number), Math.fround(message[name] as number), line);
      else deepStrictEqual(decoded[name], message[name], line);
    }
    checked += 1;
  }
  strictEqual(checked > 30, true, `only ${checked} vectors checked`);
});

test("a short payload is an error, not a partial message", () => {
  let failed = false;
  try { decode("gfx/clear", Uint8Array.of(1, 2)); } catch { failed = true; }
  strictEqual(failed, true);
});
