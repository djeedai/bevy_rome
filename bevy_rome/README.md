# 🏛️ Bevy Rome

📦 `bevy_rome`

[_Status_: 🚧 Work in progress...](#status)

## What

An experimental message passing and `Reflect`-based diff library. Enables creating a diff between 2 instances of a `Reflect`'ed type, serialize that diff for transport (in-process, networking, ...), deserialize it and apply it back to an instance to obtain back the other instance.

## Why

This allows building a data-driven Editor core which does not need any specific knowledge of custom Game types, and instead works exclusively on dynamically-typed data blobs (diffs). This is a central and foundational piece of the Editor.

## Status

🚧 _Work in progress..._

Currently trying to figure out how to generate diffs from `Reflect` objects in an efficient way; this possibly requires modifying Bevy to implement a new `Diff` trait directly in it.
