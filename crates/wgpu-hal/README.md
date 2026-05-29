# `wgpu_hal`: a cross-platform unsafe graphics abstraction

## ori_term local patches

This is a vendored fork of `wgpu-hal` 28.0.0 with the following ori_term-local patches. Each patch is documented inline with a comment header in the affected source file.

### `src/dx12/mod.rs` — DXGI swap-chain scaling per-target

DXGI's `Scaling` field on `DXGI_SWAP_CHAIN_DESC1` defaults to `DXGI_SCALING_STRETCH` (upstream wgpu-hal default). On Windows, this causes the DWM compositor to scale the previous frame to the new window size during interactive drag-resize, producing visible text jitter and stretching until the app presents the next frame.

The original ori_term patch (commit `c80fa26e`, 2026-04-01) changed `Scaling: DXGI_SCALING_STRETCH` to `Scaling: DXGI_SCALING_NONE` globally. This eliminated the resize-jitter artifact on the `CreateSwapChainForHwnd` path but inadvertently broke the `CreateSwapChainForComposition` path: Microsoft's documentation for `IDXGIFactory2::CreateSwapChainForComposition` explicitly states *"You must also specify the DXGI_SCALING_STRETCH value in the Scaling member of DXGI_SWAP_CHAIN_DESC1"* (see [https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_2/nf-dxgi1_2-idxgifactory2-createswapchainforcomposition](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_2/nf-dxgi1_2-idxgifactory2-createswapchainforcomposition)). Setting `NONE` on composition swap chains returned `E_INVALIDARG`, which surfaced as the `Surface::configure` "Invalid surface" panic and crashed `oriterm.exe` on startup whenever `window.opacity < 1.0` triggered the DComp path.

The current patch (BUG-06-108, 2026-05-27) makes the scaling per-target:

| `SurfaceTarget` variant | `Scaling` value | Why |
|---|---|---|
| `WndHandle(_)` | `DXGI_SCALING_NONE` | Routes to `CreateSwapChainForHwnd`; fixes resize jitter per `c80fa26e` |
| `Visual(_)` | `DXGI_SCALING_STRETCH` | Routes to `CreateSwapChainForComposition`; MS-mandated |
| `VisualFromWndHandle { .. }` | `DXGI_SCALING_STRETCH` | Routes to `CreateSwapChainForComposition`; MS-mandated |
| `SurfaceHandle(_)` | `DXGI_SCALING_STRETCH` | Routes to `CreateSwapChainForCompositionSurfaceHandle`; composition-class, follows STRETCH convention |
| `SwapChainPanel(_)` | `DXGI_SCALING_STRETCH` | Composition-class (XAML SwapChainPanel); follows STRETCH convention |

See `bug-tracker/plans/completed/BUG-06-108/` for the full investigation and fix consensus. Upstream wgpu-hal currently uses `DXGI_SCALING_STRETCH` everywhere — this patch's intent is to keep the resize-jitter fix on HWND without breaking the composition path.

---

## Upstream description

This crate defines a set of traits abstracting over modern graphics APIs,
with implementations ("backends") for Vulkan, Metal, Direct3D, and GL.

`wgpu_hal` is a spiritual successor to
[gfx-hal](https://github.com/gfx-rs/gfx), but with reduced scope, and
oriented towards WebGPU implementation goals. It has no overhead for
validation or tracking, and the API translation overhead is kept to the bare
minimum by the design of WebGPU. This API can be used for resource-demanding
applications and engines.

The `wgpu_hal` crate's main design choices:

- Our traits are meant to be *portable*: proper use
  should get equivalent results regardless of the backend.

- Our traits' contracts are *unsafe*: implementations perform minimal
  validation, if any, and incorrect use will often cause undefined behavior.
  This allows us to minimize the overhead we impose over the underlying
  graphics system. If you need safety, the [`wgpu-core`] crate provides a
  safe API for driving `wgpu_hal`, implementing all necessary validation,
  resource state tracking, and so on. (Note that `wgpu-core` is designed for
  use via FFI; the [`wgpu`] crate provides more idiomatic Rust bindings for
  `wgpu-core`.) Or, you can do your own validation.

- In the same vein, returned errors *only cover cases the user can't
  anticipate*, like running out of memory or losing the device. Any errors
  that the user could reasonably anticipate are their responsibility to
  avoid. For example, `wgpu_hal` returns no error for mapping a buffer that's
  not mappable: as the buffer creator, the user should already know if they
  can map it.

- We use *static dispatch*. The traits are not
  generally object-safe. You must select a specific backend type
  like [`vulkan::Api`] or [`metal::Api`], and then use that
  according to the main traits, or call backend-specific methods.

- We use *idiomatic Rust parameter passing*,
  taking objects by reference, returning them by value, and so on,
  unlike `wgpu-core`, which refers to objects by ID.

- We map buffer contents *persistently*. This means that the buffer
  can remain mapped on the CPU while the GPU reads or writes to it.
  You must explicitly indicate when data might need to be
  transferred between CPU and GPU, if `wgpu_hal` indicates that the
  mapping is not coherent (that is, automatically synchronized
  between the two devices).

- You must record *explicit barriers* between different usages of a
  resource. For example, if a buffer is written to by a compute
  shader, and then used as and index buffer to a draw call, you
  must use [`CommandEncoder::transition_buffers`] between those two
  operations.

- Pipeline layouts are *explicitly specified* when setting bind
  group. Incompatible layouts disturb groups bound at higher indices.

- The API *accepts collections as iterators*, to avoid forcing the user to
  store data in particular containers. The implementation doesn't guarantee
  that any of the iterators are drained, unless stated otherwise by the
  function documentation. For this reason, we recommend that iterators don't
  do any mutating work.

Unfortunately, `wgpu_hal`'s safety requirements are not fully documented.
Ideally, all trait methods would have doc comments setting out the
requirements users must meet to ensure correct and portable behavior. If you
are aware of a specific requirement that a backend imposes that is not
ensured by the traits' documented rules, please file an issue. Or, if you are
a capable technical writer, please file a pull request!

[`wgpu-core`]: https://crates.io/crates/wgpu-core
[`wgpu`]: https://crates.io/crates/wgpu
[`vulkan::Api`]: vulkan/struct.Api.html
[`metal::Api`]: metal/struct.Api.html

## Primary backends

The `wgpu_hal` crate has full-featured backends implemented on the following
platform graphics APIs:

- Vulkan, available on Linux, Android, and Windows, using the [`ash`] crate's
  Vulkan bindings. It's also available on macOS, if you install [MoltenVK].

- Metal on macOS, using the [`metal`] crate's bindings.

- Direct3D 12 on Windows, using the [`windows`] crate's bindings.

[`ash`]: https://crates.io/crates/ash
[MoltenVK]: https://github.com/KhronosGroup/MoltenVK
[`metal`]: https://crates.io/crates/metal
[`windows`]: https://crates.io/crates/windows

## Secondary backends

The `wgpu_hal` crate has a partial implementation based on the following
platform graphics API:

- The GL backend is available anywhere OpenGL, OpenGL ES, or WebGL are
  available. See the [`gles`] module documentation for details.

[`gles`]: gles/index.html

You can see what capabilities an adapter is missing by checking the
[`DownlevelCapabilities`][tdc] in [`ExposedAdapter::capabilities`], available
from [`Instance::enumerate_adapters`].

The API is generally designed to fit the primary backends better than the
secondary backends, so the latter may impose more overhead.

[tdc]: wgt::DownlevelCapabilities

## Debugging

Most of the information on the wiki [Debugging wgpu Applications][wiki-debug]
page still applies to this API, with the exception of API tracing/replay
functionality, which is only available in `wgpu-core`.

[wiki-debug]: https://github.com/gfx-rs/wgpu/wiki/Debugging-wgpu-Applications

