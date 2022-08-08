# Bevy Editor

This document describes a Bevy Editor vision, in an attempt to drive and focus discussions, and collate ideas and already-discussed topics for future reference. This is _not_ an official document, but a community-driven one.

## Table of Content

- [Terminology](#terminology)
- [Capabilities](#capabilities)
  - [Scene creation](#scene-creation)
  - [Interactive editing](#interactive-editing)
  - [Live editing](#live-editing)
  - [Asset baking](#asset-baking)
- [Non-goals](#non-goals)
  - [Text editor](#text-editor)
  - [In-game editor](#in-game-editor)
- [Architecture](#architecture)
  - [Vision](#vision)
  - [Core](#core)
    - [Data model](#data-model)
    - [Undo and redo](#undo-and-redo)
    - [Transport](#transport)
    - [Storage](#storage)
  - [Extensions](#extensions)
    - [Asset import](#asset-import)
    - [Hierarchy editor](#hierarchy-editor)
    - [Inspector](#inspector)
    - [Build pipeline](#build-pipeline)
- [Technical choices and challenges](#technical-choices-and-challenges)
  - [Type serialization](#type-serialization)
  - [Reflection-based diffs](#reflection-based-diffs)
  - [Hot reloading](#hot-reloading)
  - [Binary distribution](#binary-distribution)
  - [User interface](#user-interface)
  - [Async integration](#async-integration)
  - [Embedded rendering](#embedded-rendering)

## Terminology

- **Game** : An application based on the `bevy` crate. This can be a game in the proper sense, or can be any multimedia / interactive application, including CAD, architecture, etc. In this document we use the term "game" as a shortcut for any such application.
- **Editor** : The unique application this document describes the design of.
- **Author** : The user of the Editor, which aims at creating a Game by making use of the features provided by the Editor. This includes all technical professions involved in the making of the Game. In particular, this includes any non-developer user.
- **Developers** : The subset of Authors who are familiar and comfortable with writing Rust code. Some Editor functionalities around Game code handling are target toward Developers only.
- **Scene** : A serialized collection of entities and components grouped together as a hierarchy, which can be created and edited by the Editor, and loaded into the Game.
- **Game Asset** : Any asset (serialized data saved to file) referenced by the Game and needed for the Game to run, including both Editor-specific formats like Scene files or any other engine-specific asset types (polygonal meshes, animations, _etc._), as well as more general file formats like audio files, video files, image files, common 3D object formats (glTF, OBJ, _etc._), ...
- **Editor Extension** : A piece of code written by an Author to extend the functionalities of the Editor to the goals of the Game. The most common example is a custom Component wide Game-specific logic, which can be attached to an Entity in a Scene. Note that we avoid the term _plugin_ to talk about an Extension in order to prevent confusion with the `Plugin` trait of Bevy, even though in practice Extensions likely define one or more plugins that the Editor will make use of.

## Capabilities

### Scene creation

_Create and edit Scene objects, save them to storage, and load them back._

The Editor allows creating new scenes, editing them, saving them to file in an Editor-specific format, and re-loading them to continue working on them later.

### Interactive editing

_Allow interactive previews of editing actions._

The Editor favors _interactive_ editing. Wherever possible, actions from the Author must be reflected in real-time (or as near as possible). In particular, the Editor provides a way to dynamically place entities in a 3D space and move them with the mouse cursor.

### Live editing

_Enable live game editing._

The Editor can _connect_ to an instance of the Game (separate process), and supports some limited editing operations. The full range of Editor features is _not_ available in this mode, due to the complexity it would incur.

This capability is likely complex to implement efficiently, and might be a stretch goal / not available in a first version.

### Asset baking

_Enable asset baking into platform-specific format(s) optimized for runtime._

The Editor integrates with an asset pipeline to allow transforming ("baking") the Game Assets into a format optimized for a target platform. This baking operation is a one-way process; baked assets cannot be un-baked back into their editing format.

The [Distill](https://github.com/amethyst/distill) crate from Amethyst appears to be a very serious candidate for this task, given its large number of available features covering most (all?) the Editor needs and more.

## Non-goals

### Text editor

Although the Editor must allow the Author to create new components beyond the built-in Bevy ones, and instantiate those components in a Scene, the Editor does not aim at being an all-purpose text editor. Powerful text editors exist and are commonly used, with a large number of features and customizations that would be impossible to replicate inside the Editor.

### In-game editor

The Editor does not aim at being an _in-game editor_, a game-specific editor allowing the users of the game (the players) limited editing capabilities to build or edit game levels. Such an in-game editor _can_ be built by the Author, but the Editor audience is exclusively game Authors, not players (the end users of the Game).

## Architecture

### Vision

_This section describes the ideal Editor vision, and includes elements which might be forward looking and unrealistic for a first version given their complexity or the amount of work needed. They are described anyway for the sake of given an overall description of the goal._

Bevy is built as a minimal core and a collection of crates providing specific features (renderer, UI, _etc._). From a design perspective, there is no real distinction between a built-in crate and a third-party one. This approach allows efficient collaboration, enabling all users to augment the engine with their own feature(s), optionally contributing them back to the community first as third-party crates, and eventually as built-in ones. The "third-party" vs. "built-in" distinction here is mainly a crate governance one, with a minimal (while Bevy is not stable) added "seal-of-approval" kind of assurance provided by the "built-in" status.

The Editor embraces this design by providing a _Core_ around which features are built. Features are provided by _Extensions_, pieces of code communicating with the Core to exchange data. The main responsibility of the Core is to be a hub for this data flow, providing a centralized source of truth for all Extensions. This design is very similar to [The Truth](https://ourmachinery.com/post/the-story-behind-the-truth-designing-a-data-model/) from the "Our Machinery" game engine. To achieve this function, the Core defines a data model and an API to manipulate it.

The Editor itself is also a Bevy application; it depends on the `bevy` crate and builds upon the Bevy engine. This allows dogfooding Bevy to make sure features are relevant and the engine is usable and stable. The Editor however is not a typical Game, therefore might not use a typical Bevy Game architecture. In particular, it is expected that it leans more heavily on `async` and futures, where a typical Game has minimal to no use of these.

To provide the best possible editing experience and shortest iteration time, the Editor must allow running the Game in the simplest and fastest possible way. The Unity3D game engine provides this functionality, but runs a slightly different version of the Game using a custom Mono runtime embedded in the Editor, as opposed to various UnityPlayer implementations depending on platforms, some of them transpiling the C# into C++ via IL2CPP and therefore running a somewhat different code. In contrast, the goal of the Editor is to run the actual Game itself. There are various possible technical solutions to this, like running a separate process whose window is embedded into the Editor's window, or running an in-process copy of the game. These options need to be explored to find a right match.

To successfully write a Game, Developers need to make available to the Editor their own custom components and systems which enable the specific behaviors of their Game. This requires the Editor to be able to load their code, register the components and other types present in the Developer code, and instantiate such components in a Scene or any other editing context. The Editor consumes custom Developer code written in Rust, and allows the Developer to load their code without having to rebuilt the Editor from sources. This requires the Editor to deal with dynamic libraries. In the simplest variant of this feature, dynamic libraries provided by the Developer are loaded when the Editor starts, and the Developer needs to restart the Editor after each change to their code (cold reload).

Like for other Authors, enabling fast iteration for Developers is critical. To that end, the Editor must be able to hot-reload code, the process by which the Editor can unload a old version of the Developer code and reload a newly-built version containing changes without the Editor process itself being terminated. Due to the dynamic nature of the process, this feature is extremely challenging to implement efficiently and robustly, but must be considered upfront to design systems capable of handling such dynamism.

### Core

#### Data model

_Prototype:_ [📦 `bevy_rome`](../bevy_rome/)

The data model is described at length in [the "Editor data model" RFC](https://github.com/bevyengine/rfcs/pull/62).

#### Undo and redo

TODO: describe the undo/redo system, at the center of the Core architecture, making it available to all Extensions transparently, and explain the link with diffs (see [Reflection-based diffs](#reflection-based-diffs))

#### Transport

TODO: describe the serialization/deserialization via Serde into the custom in-memory data model format, with backing to disk (RON-like), and the transport abstraction which allows working with local shared memory (solo editing, performant) or with real networking (multi-user editing, remote editing).

#### Storage

TODO: describe the on-disk format (RON-like) for all assets saved by the Editor (the editing version, _not_ the baked version) and the `Serializer` / `Deserializer` implementation for Serde compatibility. talk about `git` and version control, and making the format nice to it (think about how git does merges _etc._ and try to reduce likelihood of merge conflicts and mismerges?)

### Extensions

TODO: talk about the need to try to have each Extension deal with a specific set of components, and possibly avoid overlaps as much as possible for the sake of 1) performance (parallelism) and 2) correctness (makes the job of the Core easier if less concurrent accesses).

#### Asset import

TODO: describe the asset importing Extension(s?) allowing to import into the Editor various formats like `.png`, `.obj`, `.wav`, _etc._

#### Hierarchy editor

(or Scene Editor ?)

TODO: describe the Extension which allows editing a scene by instantiating entities and components, arranging them into a hierarchy of transforms, and possibly other hierarchies. here there's a lot of room for innovation, as traditionally game engines only have a single canonical transform-based spatial hierarchy so we have an opportunity to provide something much more powerful and practical.

#### Inspector

TODO: describe the UI Extension allowing to edit the data of a selected or set of selected object(s), similar to the Unity Inspector window.

#### Build pipeline

TODO: describe the Extension integrating Distill to provide asset baking, and the (same or separate) Extension that builds/bakes/packages an Editor scene into a runtime game.

## Technical choices and challenges

### Type serialization

TODO: talk about the type registry, type IDs, the difficulty of unicity across processes with potentially different Rust versions

### Reflection-based diffs

_Prototypes:_ [📦 `bevy_rome`](../bevy_rome/), Cart's [old Diff PR](https://github.com/bevyengine/bevy/pull/944) for `bevy_reflect`

TODO: talk about the necessary changes and additions to `bevy_reflect` to enable all `Reflect`-ed types to be diffable and therefore editable by the Editor. explain the performance challenges of efficient diff create (d = b - a) and apply (a + d = b). Talk about _diffs_ (two-way) vs. _patches_ (one-way).

TODO: see also [ezEngine](https://github.com/ezEngine/ezEngine/blob/3c34b8334d0e88a5bd127c264c7b34dd60025bf3/Code/Engine/Foundation/Serialization/AbstractObjectGraph.h#L75) and its graph data model and diff handling.

### Hot reloading

_This is a stretch feature, likely not available in a first Editor version._

TODO: explain hot-reloading of code and its challenges around DLL unloading, like TypeId mismatch. talk about `dylib` vs. `cdylib`, the limitations of `dylib` with respect to compiler toolchain version and unstable ABI, and the limitations of `cdylib` with respect to the C interface and lack of support for all Rust features.

See how [Unreal Editor itself discourages the use of hot reloading](https://unrealcommunity.wiki/live-compiling-in-unreal-projects-tp14jcgs) due to bugs and asset corruptions:

> Most users recommend avoiding Hot Reloading entirely, which means you need to close the editor to compile safely.

and

> If you initiate a Hot Reload, don't panic. Just make sure to close the editor without saving anything, run Build in your IDE, and carry on.

### Binary distribution

The Editor is first and foremost distributed as a prebuilt binary application. Building from source is supported, but shall not be the main consumption path, to avoid a high barrier to entry for Authors unfamiliar with Rust and Cargo.

The release process shall be automated via a CI pipeline on GitHub, which builds the Editor and packs it with any additional dependent file (images, icons, _etc._) into an archive (ZIP), or possibly even an installer (`.msi`, `.deb`, _etc._). This allows frequent releases, possiby nightly, and reduces the overhead of each release by limiting the number of manual steps, while ensuring consistency and allowing gating from automated testing to ensure quality.

### User interface

There is a strong desire to build the user interface (UI) of the Editor with the Bevy UI itself. Currently the UI provided by the `bevy_ui` crate is too cumbersome and limited to be able to stand up a full-featured user interface for the Editor. In particular, the constructs are too low-level and verbose to be readily usable. `bevy_ui` also focuses heavily on the rendering part of the UI, and lacks much in other areas like consistent user input and interaction, or localization.

Various alternatives have been proposed, like `egui`, but their integration with Bevy might cause more friction than it helps, and prevents the Editor from dogfooding the Bevy UI, provide valuable feedback on it, and drive its development. Therefore a solution based on Bevy itself, either by improving `bevy_ui` or by writing a new crate focusing on widget-intensive Editor-like applications.

Looking at the Rust UI ecosystem, one valuable contribution has been the [Druid](https://github.com/linebender/druid) UI framework. Although its design and data model do not necessarily fit well with Bevy to enable directly consuming the crate, an important learning is its ability to empower developers with a 2D graphics library and UI framework allowing them to build a polished and themed custom widget in a matter of minutes without the need to bother with GPU contructs (meshes, textures, _etc._). Instead `druid` provides a set of 2D drawing commands via a [`RenderContext` trait](https://docs.rs/piet/latest/piet/trait.RenderContext.html), and a [`Widget` trait](https://docs.rs/druid/latest/druid/trait.Widget.html) with various user interaction methods to implement, which together make the process straightforward. To that respect, the experience is similar to (but the implementation much different, more efficient) the immediate-mode GUI of Unity3D, which helped popularize the game engine and is one of its leading selling points for Editor customizing. Due to the wide variety of widgets required for the Editor, a similar solution should be investigated, for example by providing a backend based on the Bevy renderer for [Piet](https://github.com/linebender/piet), the 2D library used by Druid (see [this blog post](https://raphlinus.github.io/rust/graphics/2018/10/11/2d-graphics.html) by Raph Levien describing Piet).

### Async integration

Being a "Desktop application" rather than a Game, it is expected the Engine will more heavily lean toward the use of asynchronous operations and futures. This will likely require better support for those features than already available today in Bevy. At the minute, the async integration in Bevy appears almost anecdotical; with the Editor, it could and should become a valid and robust design alternative.

### Embedded rendering

Embedding the rendering of the Game process into the UI of the Editor is made possible by leveraging the platform's compositor API. This feature is available on most platforms.

This technique has some major advantages. First, it is robust to game crashes, avoiding loss of data as long as the Editor itself remains stable. It also naturally lends itself to networking, allowing video streaming of a Game running _e.g._ on a DevKit (console) or any other remote hardware. Data-wise, this hints at an Editor design where the build pipeline runs in the background and processes asset changes on the fly, to minimize the delay between the time the Author presses the Play button and the time the Game actually runs. A possible alternative is to use _editing assets_ (non-baked assets) by providing the Game with a plugin which allows loading such assets during development; however this diverges the "development Game" from the "shipped Game", which is generally preferable to minimize as much as possible. For networked play, this also means either creating a networked asset fileserver so the remote Game host can access the editing assets from the local Editor host, or creating a deploy step to copy those assets (which can be slow, especially when working remotely).

Generally however embedding rendering is a challenging feature to achieve, to make it work smoothly and integrate it with the Editor UI without quirks. This requires the UI to be designed to allow embedding native child windows, something existing established UI frameworks (Qt, GTK, ...) support to various extents ranging from no support at all to complete seemless integration with their own widgets. This feature will likely require some prototyping in Rust, and some changes and improvements to the Bevy UI.
