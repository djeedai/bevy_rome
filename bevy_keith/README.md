# 🐕 Bevy Keith

📦 `bevy_keith`

[_Status_: 🚧 Work in progress...](#status)

👀 See also [the crate's own `README.crate.md`](README.crate.md) for details on the crate content.

## What

2D graphic library inspired by Piet (📦 [`piet`](https://crates.io/crates/piet)), with better integration into Bevy.

Like the [🎨 Bevy Piet](../bevy_piet/) experiment, the 📦 `bevy_keith` crate exposes a `RenderContext`trait for 2D graphics drawning, implemented in terms of a single-draw-call Bevy `Transparent2D` render pass.

The crate exposes a `Canvas` component giving access to the `RenderContext` for that canvas, and rendering any content drawn to that context into the 2D render pipeline of Bevy, in an immediate-mode way (transient primitives are not saved over frames, are instead immediately consumed for the current frame then discarded). However, the design allows later adding support for saving the content of a `Canvas`, which could be exposed to achieve caching in a drawing-intensive UI like the one an Editor typically exhibits.

The primitives are saved as is in a "primitive buffer" (storage buffer), later indexed by a custom bitmask index containing the primitive offset and additional data. A single unified shader draws all kinds of primitives, allowing to draw an entire `Canvas` (and possibly multiple with dynamic batching, if the canvas is not saved) with a single draw call.

Text rendering uses a custom text pipeline inspired by Bevy's own text pipeline, but storing all glyphs of all font sizes mixed into a shared atlas. This approach reduces the number of textures to bind, making it more likely that the entire drawing be rendered in a single draw call. Currently the text pipeline still uses pre-rastered glyphs, so may suffer from aliasing.

## Why

Same as [🎨 Bevy Piet](../bevy_piet/), this allows easily drawing 2D graphics, and in particular UI widgets, with a higher-level API than the one Bevy proposes, and a lot more dynamism (animated controls like changing color on hover, adding a border on focus, drag-and-drop resizing, ...).

Freeing ourselves from the Piet interface however allows a better integration with Bevy, and to limit the scope of the primitives supported to those of interest for UI, allowing for faster prototyping.

## Status

🚧 _Work in progress..._

This is a continuation of the [🎨 Bevy Piet](../bevy_piet/) experiment.

Currently a base SDF renderer is implemented to draw axis-aligned rectangles (with or without texturing), lines with variable thickness, and texts.
