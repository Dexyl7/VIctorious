//! Contrast-adaptive sharpening of the frame we show (`VICTORY_SHARPEN`,
//! a strength from 0 to 1; off when unset or 0).
//!
//! The Series S profile draws the world at 720p and the title's own
//! upscaler makes 1080p of it: softer than the console's 1080p would be.
//! The driver's sharpening does not reach our window (tested: no change at
//! 100 %), so the frame is sharpened here, as the last step before it is
//! copied to the swap chain. Each pixel is pushed away from its four
//! neighbours by an amount that falls where the neighbourhood already has
//! strong contrast (edges are not over-sharpened, flat areas get the most),
//! the usual contrast-adaptive approach. One compute pass at the size of
//! the frame: well under a millisecond at 1080p.

use std::sync::Mutex;

use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::ID3DBlob;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;

/// Presents in flight whose descriptors must not be rewritten (two each).
const SLOTS: u32 = 16;

const SOURCE: &str = r#"
#define RS "RootConstants(num32BitConstants=3, b0), DescriptorTable(SRV(t0), UAV(u0))"
cbuffer Size : register(b0) { uint w; uint h; float strength; };
Texture2D<float4> src : register(t0);
RWTexture2D<float4> outp : register(u0);
float3 at(int2 p) { return src.Load(int3(clamp(p, int2(0, 0), int2(w - 1, h - 1)), 0)).rgb; }
[RootSignature(RS)]
[numthreads(8, 8, 1)]
void main(uint3 id : SV_DispatchThreadID) {
    if (id.x >= w || id.y >= h) return;
    int2 p = int2(id.xy);
    float3 e = at(p);
    float3 n = at(p + int2(0, -1));
    float3 s = at(p + int2(0, 1));
    float3 wv = at(p + int2(-1, 0));
    float3 ea = at(p + int2(1, 0));
    // The neighbourhood's range: little room to either end means an edge
    // already, which is sharpened less.
    float3 lo = min(e, min(min(n, s), min(wv, ea)));
    float3 hi = max(e, max(max(n, s), max(wv, ea)));
    float3 room = saturate(min(lo, 1.0 - hi) / max(hi, 1e-4));
    float3 amount = sqrt(room);
    // Negative lobe: -1/8 (soft) to -1/5 (strong).
    float3 weight = amount * (-1.0 / lerp(8.0, 5.0, saturate(strength)));
    float3 sharpened = (e + (n + s + wv + ea) * weight) / (1.0 + 4.0 * weight);
    float a = src.Load(int3(p, 0)).a;
    outp[id.xy] = float4(saturate(sharpened), a);
}
"#;

struct Pass {
    root: ID3D12RootSignature,
    pso: ID3D12PipelineState,
    views: ID3D12DescriptorHeap,
    cpu: usize,
    gpu: u64,
    step: u32,
    next: u32,
    /// The texture shown instead, by size and format.
    out: Option<(u64, u32, i32, ID3D12Resource)>,
}
// SAFETY: D3D12 objects are free-threaded; the pass is used under a lock.
unsafe impl Send for Pass {}

static PASS: Mutex<Option<Option<Pass>>> = Mutex::new(None);

/// The strength asked for, if any (0 < strength <= 1).
pub(crate) fn strength() -> Option<f32> {
    static STRENGTH: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    *STRENGTH.get_or_init(|| {
        let s = std::env::var("VICTORY_SHARPEN").ok()?.trim().parse::<f32>().ok()?;
        (s > 0.0).then_some(s.min(1.0))
    })
}

fn make(device: &ID3D12Device) -> windows::core::Result<Pass> {
    let mut code: Option<ID3DBlob> = None;
    // SAFETY: the source and names live through the call; the out pointer
    // is ours.
    unsafe {
        D3DCompile(
            SOURCE.as_ptr().cast(),
            SOURCE.len(),
            windows::core::s!("sharpen"),
            None,
            None,
            windows::core::s!("main"),
            windows::core::s!("cs_5_1"),
            0,
            0,
            &mut code,
            None,
        )?;
    }
    let code = code.ok_or_else(windows::core::Error::empty)?;
    // SAFETY: the blob's own bytes (the root signature is inside the
    // shader); plain object creation and queries.
    unsafe {
        let bytes = std::slice::from_raw_parts(code.GetBufferPointer().cast::<u8>(), code.GetBufferSize());
        let root: ID3D12RootSignature = device.CreateRootSignature(0, bytes)?;
        let pso: ID3D12PipelineState = device.CreateComputePipelineState(&D3D12_COMPUTE_PIPELINE_STATE_DESC {
            pRootSignature: std::mem::transmute_copy(&root),
            CS: D3D12_SHADER_BYTECODE {
                pShaderBytecode: bytes.as_ptr().cast(),
                BytecodeLength: bytes.len(),
            },
            ..Default::default()
        })?;
        let kind = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
        let views: ID3D12DescriptorHeap = device.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
            Type: kind,
            NumDescriptors: SLOTS * 2,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
            NodeMask: 0,
        })?;
        Ok(Pass {
            cpu: views.GetCPUDescriptorHandleForHeapStart().ptr,
            gpu: views.GetGPUDescriptorHandleForHeapStart().ptr,
            step: device.GetDescriptorHandleIncrementSize(kind),
            root,
            pso,
            views,
            next: 0,
            out: None,
        })
    }
}

/// The typed format views of a frame in `format` read and write, if this
/// pass handles it: 8-bit RGBA, 10-bit RGB, 16-bit float RGBA.
fn typed(format: DXGI_FORMAT) -> Option<DXGI_FORMAT> {
    match format.0 {
        27 | 28 => Some(DXGI_FORMAT_R8G8B8A8_UNORM),
        23 | 24 => Some(DXGI_FORMAT_R10G10B10A2_UNORM),
        9 | 10 => Some(DXGI_FORMAT_R16G16B16A16_FLOAT),
        _ => None,
    }
}

/// Records, into the present's `list`, `frame` (in the copy-source state,
/// left in it) sharpened into a texture of ours, and gives that texture,
/// left in the copy-source state. `None` (nothing recorded) without a
/// strength, for a format this pass does not handle, or if it could not
/// be made.
pub(crate) fn record(device: &ID3D12Device, list: &ID3D12GraphicsCommandList, frame: &ID3D12Resource) -> Option<ID3D12Resource> {
    let strength = strength()?;
    // SAFETY: a live resource; a plain query.
    let desc = unsafe { frame.GetDesc() };
    let Some(format) = typed(desc.Format) else {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| tracing::warn!(format = desc.Format.0, "the frame is not sharpened: a format this pass does not handle"));
        return None;
    };
    let mut kept = PASS.lock().unwrap_or_else(|e| e.into_inner());
    if kept.is_none() {
        *kept = Some(match make(device) {
            Ok(p) => {
                tracing::info!(strength, "the frame we show is sharpened (VICTORY_SHARPEN)");
                Some(p)
            }
            Err(e) => {
                tracing::warn!(error = %e, "the sharpening pass could not be made");
                None
            }
        });
    }
    let pass = kept.as_mut()?.as_mut()?;
    let (w, h) = (desc.Width, desc.Height);
    if pass.out.as_ref().is_none_or(|o| (o.0, o.1, o.2) != (w, h, format.0)) {
        let heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_DEFAULT,
            ..Default::default()
        };
        let out_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Width: w,
            Height: h,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
            ..Default::default()
        };
        let mut out: Option<ID3D12Resource> = None;
        // SAFETY: plain creation of a device-local texture.
        unsafe {
            device
                .CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &out_desc, D3D12_RESOURCE_STATE_COPY_SOURCE, None, &mut out)
                .ok()?;
        }
        pass.out = Some((w, h, format.0, out?));
    }
    let out = pass.out.as_ref()?.3.clone();
    let first = (pass.next % SLOTS) * 2;
    pass.next = pass.next.wrapping_add(1);
    let cpu = |i: u32| D3D12_CPU_DESCRIPTOR_HANDLE { ptr: pass.cpu + ((first + i) * pass.step) as usize };
    let gpu = D3D12_GPU_DESCRIPTOR_HANDLE { ptr: pass.gpu + u64::from(first * pass.step) };
    let srv = D3D12_SHADER_RESOURCE_VIEW_DESC {
        Format: format,
        ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
        Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
            Texture2D: D3D12_TEX2D_SRV {
                MostDetailedMip: 0,
                MipLevels: 1,
                ..Default::default()
            },
        },
    };
    let uav = D3D12_UNORDERED_ACCESS_VIEW_DESC {
        Format: format,
        ViewDimension: D3D12_UAV_DIMENSION_TEXTURE2D,
        Anonymous: D3D12_UNORDERED_ACCESS_VIEW_DESC_0 {
            Texture2D: D3D12_TEX2D_UAV { MipSlice: 0, PlaneSlice: 0 },
        },
    };
    let read = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
    let t = crate::host::transition;
    let before = [
        t(frame, D3D12_RESOURCE_STATE_COPY_SOURCE, read),
        t(&out, D3D12_RESOURCE_STATE_COPY_SOURCE, D3D12_RESOURCE_STATE_UNORDERED_ACCESS),
    ];
    let after = [
        t(frame, read, D3D12_RESOURCE_STATE_COPY_SOURCE),
        t(&out, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_COPY_SOURCE),
    ];
    let constants: [u32; 3] = [w as u32, h, strength.to_bits()];
    // SAFETY: an open list (the present's); slots of our heap; live
    // resources and objects (the pass's own live as long as the process,
    // the texture is kept by the present's list).
    unsafe {
        device.CreateShaderResourceView(frame, Some(&srv), cpu(0));
        device.CreateUnorderedAccessView(&out, None::<&ID3D12Resource>, Some(&uav), cpu(1));
        list.ResourceBarrier(&before);
        list.SetDescriptorHeaps(&[Some(pass.views.clone())]);
        list.SetComputeRootSignature(&pass.root);
        list.SetPipelineState(&pass.pso);
        list.SetComputeRoot32BitConstants(0, 3, constants.as_ptr().cast(), 0);
        list.SetComputeRootDescriptorTable(1, gpu);
        list.Dispatch((w as u32).div_ceil(8), h.div_ceil(8), 1);
        list.ResourceBarrier(&after);
    }
    Some(out)
}
