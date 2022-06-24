# Bevy Rome

A collection of prototype crates toward a Bevy Editor.

- [🏛️ Bevy Rome](#🏛️-bevy-rome)
- [🎨 Bevy Piet](#🎨-bevy-piet)

## 🏛️ Bevy Rome

📦 `bevy_rome`

### What

An experimental message passing and `Reflect`-based diff library. Enables creating a diff between 2 instances of a `Reflect`'ed type, serialize that diff for transport (in-process, networking, ...), deserialize it and apply it back to an instance to obtain back the other instance.

### Why

This allows building a data-driven Editor core which does not need any specific knowledge of custom Game types, and instead works exclusively on dynamically-typed data blobs (diffs). This is a central and foundational piece of the Editor.

## 🎨 Bevy Piet

📦 `bevy_piet`

### What

Adapter crate for the Piet 2D graphic abstraction (📦 [`piet`](https://crates.io/crates/piet)) to expose a [`RenderContext`](https://docs.rs/piet/latest/piet/trait.RenderContext.html) implemented in terms of a Bevy `Transparent2D` render pass. The `piet` crate is the library used by the Druid UI framework for its rendering.

The crate exposes a `PietCanvas` component giving access to the `RenderContext` for that canvas, and rendering any content drawn to that context into the 2D render pipeline of Bevy, in an immediate-mode way (transient primitives are not saved over frames, are instead immediately consumed for the current frame then discarded).

### Why

This allows easily drawing 2D graphics, and in particular UI widgets, with a higher-level API than the one Bevy proposes, and a lot more dynamism (animated controls like changing color on hover, adding a border on focus, drag-and-drop resizing, ...).
