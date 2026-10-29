// A square the arrow keys move, drawn every frame: the TypeScript
// counterpart of the Rust game template.

import { log, publish, subscribe } from "bones:extension/host-api@1.0.0";
import { decode, encode } from "./wire.ts";

/** Pixels per second. */
const SPEED = 240;
const SIZE = 40;

let x = 380;
let y = 280;
let dx = 0;
let dy = 0;

/** Sets the held direction for an arrow key; `amount` is 1 down, 0 up. */
function hold(key: string, amount: number): void {
  if (key === "Left") dx = -amount;
  if (key === "Right") dx = amount;
  if (key === "Up") dy = -amount;
  if (key === "Down") dy = amount;
}

export const extensionApi = {
  init(): void {
    subscribe("core/tick");
    subscribe("input/key-down");
    subscribe("input/key-up");
    log("info", "ready: move the square with the arrow keys");
  },

  shutdown(): void {},

  onTick(dt: number): void {
    x += dx * SPEED * dt;
    y += dy * SPEED * dt;
    publish("gfx/clear", encode("gfx/clear", { r: 20, g: 20, b: 30, a: 255 }));
    publish("gfx/draw-rect", encode("gfx/draw-rect", {
      x: Math.round(x), y: Math.round(y), w: SIZE, h: SIZE,
      filled: true, r: 80, g: 200, b: 255, a: 255, layer: 0, screen_space: false,
    }));
  },

  onMessage(topic: string, _sender: string, payload: Uint8Array): Uint8Array | undefined {
    if (topic === "input/key-down") hold(decode("input/key-down", payload).key, 1);
    if (topic === "input/key-up") hold(decode("input/key-up", payload).key, 0);
    return undefined;
  },
};
