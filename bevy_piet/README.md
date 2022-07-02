# 🎨 Bevy Piet

📦 `bevy_piet`

[_Status_: ❌ Abandonned.](#status)

## What

Adapter crate for the Piet 2D graphic abstraction (📦 [`piet`](https://crates.io/crates/piet))

The 📦 `bevy_piet` crate exposes a [`piet::RenderContext`](https://docs.rs/piet/latest/piet/trait.RenderContext.html) implemented in terms of a Bevy `Transparent2D` render pass. The `piet` crate is the library used by the Druid UI framework for its rendering.

The crate exposes a `PietCanvas` component giving access to the `RenderContext` for that canvas, and rendering any content drawn to that context into the 2D render pipeline of Bevy, in an immediate-mode way (transient primitives are not saved over frames, are instead immediately consumed for the current frame then discarded).

## Why

This allows easily drawing 2D graphics, and in particular UI widgets, with a higher-level API than the one Bevy proposes, and a lot more dynamism (animated controls like changing color on hover, adding a border on focus, drag-and-drop resizing, ...).

## Status

❌ _Abandonned._

The experiment was aimed at evaluating the Druid approach to using Piet for drawing UI widgets. The Piet interface, namely its `piet::RenderContext` trait, provide immediate mode style 2D graphics drawing. This is extremely convenient to quickly create new widgets in Druid.

The initial integration with Bevy was relatively straightforward. The implementation can draw lines and quads easily. However, this surfaced 2 issues with the interface exposed by the `piet::RenderContext` trait of Piet:

1. It's designed to plug into an existing 2D graphics library like `Direct2D` or `cairo`. Implementing such a library is a massive piece of work, especially for all things text-related, but also simply for efficiently handling a large number of 2D drawning primitives. Druid itself simply defers to platform-specific implementations, but we can't do that with Bevy as rendering needs to be handled via Bevy's own renderer.

2. Text handling is particularly hard to map to the way Bevy does things. The interface exposes a synchonous font loading method, which would require delayed loading and returning an unloaded handle. Bevy's own `Font` class, via the underlying `ab_glyph::Font` trait, is also missing a getter for retrieving the font family of a font loaded from a blob of bytes, which is required to implement the `piet::FontFamily` API.

Overall, aside from the large amount of work that would be required to finish this implementation, it also feels like the `piet::RenderContext` is getting in the way here more than it helps. The line and quad drawing can probably be reused to draw some immediate-mode Bevy UI, but it's clear text handling should directly use Bevy's own implementation, and avoid immediate-mode drawing which is expensive for fonts.