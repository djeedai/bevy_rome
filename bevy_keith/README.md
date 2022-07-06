# 🐕 Bevy Keith

📦 `bevy_keith`

[_Status_: 🚧 Work in progress...](#status)

## What

2D graphic library inspired by Piet (📦 [`piet`](https://crates.io/crates/piet)), with better integration into Bevy.

Like the [🎨 Bevy Piet](../bevy_piet/) experiment, the 📦 `bevy_keith` crate exposes a `RenderContext`trait for 2D graphics drawning, implemented in terms of a single-draw-call Bevy `Transparent2D` render pass.

The crate exposes a `Canvas` component giving access to the `RenderContext` for that canvas, and rendering any content drawn to that context into the 2D render pipeline of Bevy, in an immediate-mode way (transient primitives are not saved over frames, are instead immediately consumed for the current frame then discarded). However, the design allows later adding support for saving the content of a `Canvas`, which could be exposed to achieve caching in a drawing-intensive UI like the one an Editor typically exhibits.

The primitives are saved as is in a "primitive buffer" (storage buffer), later indexed by a custom bitmask index containing the primitive offset and additional data. A single unified shader draws all kinds of primitives, allowing to draw an entire `Canvas` (and possibly multiple with dynamic batching, if the canvas is not saved) with a single draw call.

## Why

Same as [🎨 Bevy Piet](../bevy_piet/), this allows easily drawing 2D graphics, and in particular UI widgets, with a higher-level API than the one Bevy proposes, and a lot more dynamism (animated controls like changing color on hover, adding a border on focus, drag-and-drop resizing, ...).

Freeing ourselves from the Piet interface however allows a better integration with Bevy, and to limit the scope of the primitives supported to those of interest for UI, allowing for faster prototyping.

(TODO) Text handling is also deferred entirely to Bevy, which already has functionalities for this, and all texts are cached by default due to the processing cost of dynamic text rendering each frame.

## Status

🚧 _Work in progress..._

This is a continuation of the [🎨 Bevy Piet](../bevy_piet/) experiment.

Currently a base stub is implemented to draw rectangles and lines only. Text handling is not tackled yet.
