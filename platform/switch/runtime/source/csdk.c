/* SPDX-License-Identifier: MPL-2.0
 * deko3d driver for the CrySirror wgpu custom backend (see csdk.h). It shares
 * gpu.c's device, queue and swapchain; the integrated Siglus renderer and
 * this driver are never active in the same session. */
#include "csdk.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <switch.h>
#include <deko3d.h>

void siglus_switch_log_message(const char* message);
DkDevice siglus_gpu_dk_device(void);
DkQueue siglus_gpu_dk_queue(void);
DkSwapchain siglus_gpu_dk_swapchain(void);
DkImage* siglus_gpu_dk_framebuffer(unsigned index);
unsigned siglus_gpu_dk_framebuffer_count(void);
NWindow* siglus_gpu_dk_window(void);

enum {
    MaxMem = 32768,
    MaxImages = 8192,
    ImageDescriptors = 16384,
    SamplerDescriptors = 2048,
    CodeSize = 24 * 1024 * 1024,
    MaxShaders = 1024,
    Slots = 3,
    ChunkSize = 1024 * 1024,
    MaxChunks = 128,
};

typedef struct Image {
    bool used, owned;
    DkImage image;
    DkMemBlock memory;
    uint32_t format, type, width, height, depth, mips, samples;
} Image;

typedef struct Slot {
    DkCmdBuf cmdbuf;
    DkMemBlock chunks[MaxChunks];
    uint32_t chunk_count;
    DkFence fence;
    uint64_t seq;
    bool pending;
} Slot;

static DkDevice device;
static DkQueue queue;
static DkMemBlock mems[MaxMem];
static uint32_t mem_sizes[MaxMem];
static Image images[MaxImages];
static DkMemBlock descriptor_memory;
static DkImageDescriptor* image_descriptors;
static DkSamplerDescriptor* sampler_descriptors;
static DkMemBlock code_memory;
static uint32_t code_used;
static DkShader shaders[MaxShaders];
static uint32_t shader_count;
static Slot slots[Slots];
static Slot* current;
static uint64_t next_seq = 1;
static uint64_t completed_seq;
static bool ready;
static uint32_t display_count;
static int32_t display_ids[8];

static void logf_(const char* fmt, int a, int b) {
    char m[200];
    snprintf(m, sizeof(m), fmt, a, b);
    siglus_switch_log_message(m);
}

static uint32_t align_up(uint32_t v, uint32_t a) { return (v + a - 1) & ~(a - 1); }

static DkMemBlock make_memory(uint64_t size, uint32_t flags) {
    DkMemBlockMaker maker;
    dkMemBlockMakerDefaults(&maker, device, align_up((uint32_t) size, DK_MEMBLOCK_ALIGNMENT));
    maker.flags = flags;
    return dkMemBlockCreate(&maker);
}

static DkImageFormat dk_format(uint32_t f) {
    switch (f) {
    case CsFmt_R8Unorm: return DkImageFormat_R8_Unorm;
    case CsFmt_R8Snorm: return DkImageFormat_R8_Snorm;
    case CsFmt_R8Uint: return DkImageFormat_R8_Uint;
    case CsFmt_R8Sint: return DkImageFormat_R8_Sint;
    case CsFmt_R16Float: return DkImageFormat_R16_Float;
    case CsFmt_R16Unorm: return DkImageFormat_R16_Unorm;
    case CsFmt_R16Uint: return DkImageFormat_R16_Uint;
    case CsFmt_R16Sint: return DkImageFormat_R16_Sint;
    case CsFmt_R32Float: return DkImageFormat_R32_Float;
    case CsFmt_R32Uint: return DkImageFormat_R32_Uint;
    case CsFmt_R32Sint: return DkImageFormat_R32_Sint;
    case CsFmt_Rg8Unorm: return DkImageFormat_RG8_Unorm;
    case CsFmt_Rg8Snorm: return DkImageFormat_RG8_Snorm;
    case CsFmt_Rg8Uint: return DkImageFormat_RG8_Uint;
    case CsFmt_Rg8Sint: return DkImageFormat_RG8_Sint;
    case CsFmt_Rg16Float: return DkImageFormat_RG16_Float;
    case CsFmt_Rg16Unorm: return DkImageFormat_RG16_Unorm;
    case CsFmt_Rg16Uint: return DkImageFormat_RG16_Uint;
    case CsFmt_Rg16Sint: return DkImageFormat_RG16_Sint;
    case CsFmt_Rg32Float: return DkImageFormat_RG32_Float;
    case CsFmt_Rg32Uint: return DkImageFormat_RG32_Uint;
    case CsFmt_Rg32Sint: return DkImageFormat_RG32_Sint;
    case CsFmt_Rgba8Unorm: return DkImageFormat_RGBA8_Unorm;
    case CsFmt_Rgba8UnormSrgb: return DkImageFormat_RGBA8_Unorm_sRGB;
    case CsFmt_Rgba8Snorm: return DkImageFormat_RGBA8_Snorm;
    case CsFmt_Rgba8Uint: return DkImageFormat_RGBA8_Uint;
    case CsFmt_Rgba8Sint: return DkImageFormat_RGBA8_Sint;
    case CsFmt_Bgra8Unorm: return DkImageFormat_BGRA8_Unorm;
    case CsFmt_Bgra8UnormSrgb: return DkImageFormat_BGRA8_Unorm_sRGB;
    case CsFmt_Rgba16Float: return DkImageFormat_RGBA16_Float;
    case CsFmt_Rgba16Unorm: return DkImageFormat_RGBA16_Unorm;
    case CsFmt_Rgba16Uint: return DkImageFormat_RGBA16_Uint;
    case CsFmt_Rgba16Sint: return DkImageFormat_RGBA16_Sint;
    case CsFmt_Rgba32Float: return DkImageFormat_RGBA32_Float;
    case CsFmt_Rgba32Uint: return DkImageFormat_RGBA32_Uint;
    case CsFmt_Rgba32Sint: return DkImageFormat_RGBA32_Sint;
    case CsFmt_Rgb10a2Unorm: return DkImageFormat_RGB10A2_Unorm;
    case CsFmt_Rg11b10Float: return DkImageFormat_RG11B10_Float;
    case CsFmt_Depth16: return DkImageFormat_Z16;
    case CsFmt_Depth24: return DkImageFormat_Z24X8;
    case CsFmt_Depth24Stencil8: return DkImageFormat_Z24S8;
    case CsFmt_Depth32F: return DkImageFormat_ZF32;
    case CsFmt_Depth32FStencil8: return DkImageFormat_ZF32_X24S8;
    case CsFmt_Stencil8: return DkImageFormat_S8;
    case CsFmt_Bc1: return DkImageFormat_RGBA_BC1;
    case CsFmt_Bc1Srgb: return DkImageFormat_RGBA_BC1_sRGB;
    case CsFmt_Bc2: return DkImageFormat_RGBA_BC2;
    case CsFmt_Bc2Srgb: return DkImageFormat_RGBA_BC2_sRGB;
    case CsFmt_Bc3: return DkImageFormat_RGBA_BC3;
    case CsFmt_Bc3Srgb: return DkImageFormat_RGBA_BC3_sRGB;
    case CsFmt_Bc4Unorm: return DkImageFormat_R_BC4_Unorm;
    case CsFmt_Bc4Snorm: return DkImageFormat_R_BC4_Snorm;
    case CsFmt_Bc5Unorm: return DkImageFormat_RG_BC5_Unorm;
    case CsFmt_Bc5Snorm: return DkImageFormat_RG_BC5_Snorm;
    case CsFmt_Bc6hUfloat: return DkImageFormat_RGBA_BC6H_UF16_Float;
    case CsFmt_Bc6hSfloat: return DkImageFormat_RGBA_BC6H_SF16_Float;
    case CsFmt_Bc7: return DkImageFormat_RGBA_BC7_Unorm;
    case CsFmt_Bc7Srgb: return DkImageFormat_RGBA_BC7_Unorm_sRGB;
    default: return DkImageFormat_None;
    }
}

static DkImageType dk_type(uint32_t t) {
    switch (t) {
    case CsType_2DArray: return DkImageType_2DArray;
    case CsType_Cube: return DkImageType_Cubemap;
    case CsType_CubeArray: return DkImageType_CubemapArray;
    case CsType_3D: return DkImageType_3D;
    case CsType_2DMS: return DkImageType_2DMS;
    case CsType_2DMSArray: return DkImageType_2DMSArray;
    case CsType_1D: return DkImageType_1D;
    default: return DkImageType_2D;
    }
}
static DkMsMode ms_mode(uint32_t samples) {
    return samples >= 8 ? DkMsMode_8x : samples >= 4 ? DkMsMode_4x : samples >= 2 ? DkMsMode_2x : DkMsMode_1x;
}
static DkBlendFactor dk_factor(uint8_t f) {
    switch (f) {
    case CsBf_Zero: return DkBlendFactor_Zero;
    case CsBf_One: return DkBlendFactor_One;
    case CsBf_Src: return DkBlendFactor_SrcColor;
    case CsBf_OneMinusSrc: return DkBlendFactor_InvSrcColor;
    case CsBf_SrcAlpha: return DkBlendFactor_SrcAlpha;
    case CsBf_OneMinusSrcAlpha: return DkBlendFactor_InvSrcAlpha;
    case CsBf_Dst: return DkBlendFactor_DstColor;
    case CsBf_OneMinusDst: return DkBlendFactor_InvDstColor;
    case CsBf_DstAlpha: return DkBlendFactor_DstAlpha;
    case CsBf_OneMinusDstAlpha: return DkBlendFactor_InvDstAlpha;
    case CsBf_SrcAlphaSaturated: return DkBlendFactor_SrcAlphaSaturate;
    case CsBf_Constant: return DkBlendFactor_ConstColor;
    case CsBf_OneMinusConstant: return DkBlendFactor_InvConstColor;
    default: return DkBlendFactor_One;
    }
}
static DkBlendOp dk_op(uint8_t o) {
    switch (o) {
    case CsBo_Subtract: return DkBlendOp_Sub;
    case CsBo_ReverseSubtract: return DkBlendOp_RevSub;
    case CsBo_Min: return DkBlendOp_Min;
    case CsBo_Max: return DkBlendOp_Max;
    default: return DkBlendOp_Add;
    }
}
static DkCompareOp dk_cmp(uint8_t c) {
    switch (c) {
    case CsCmp_Never: return DkCompareOp_Never;
    case CsCmp_Less: return DkCompareOp_Less;
    case CsCmp_Equal: return DkCompareOp_Equal;
    case CsCmp_LessEqual: return DkCompareOp_Lequal;
    case CsCmp_Greater: return DkCompareOp_Greater;
    case CsCmp_NotEqual: return DkCompareOp_NotEqual;
    case CsCmp_GreaterEqual: return DkCompareOp_Gequal;
    default: return DkCompareOp_Always;
    }
}
static DkStencilOp dk_sop(uint8_t s) {
    switch (s) {
    case CsSop_Zero: return DkStencilOp_Zero;
    case CsSop_Replace: return DkStencilOp_Replace;
    case CsSop_Invert: return DkStencilOp_Invert;
    case CsSop_IncrementClamp: return DkStencilOp_Incr;
    case CsSop_DecrementClamp: return DkStencilOp_Decr;
    case CsSop_IncrementWrap: return DkStencilOp_IncrWrap;
    case CsSop_DecrementWrap: return DkStencilOp_DecrWrap;
    default: return DkStencilOp_Keep;
    }
}
static DkPrimitive dk_prim(uint32_t p) {
    switch (p) {
    case CsPrim_Points: return DkPrimitive_Points;
    case CsPrim_Lines: return DkPrimitive_Lines;
    case CsPrim_LineStrip: return DkPrimitive_LineStrip;
    case CsPrim_TriangleStrip: return DkPrimitive_TriangleStrip;
    default: return DkPrimitive_Triangles;
    }
}
static DkStage dk_stage(uint32_t s) {
    return s == CsStage_Vertex ? DkStage_Vertex : s == CsStage_Fragment ? DkStage_Fragment : DkStage_Compute;
}

static void add_memory(void* user, DkCmdBuf cmdbuf, size_t min_size) {
    Slot* slot = (Slot*) user;
    if (slot->chunk_count >= MaxChunks) {
        siglus_switch_log_message("csdk: command memory exhausted\n");
        return;
    }
    const uint32_t size = align_up((uint32_t) (min_size > ChunkSize ? min_size : ChunkSize), DK_MEMBLOCK_ALIGNMENT);
    DkMemBlock memory = make_memory(size, DkMemBlockFlags_CpuUncached | DkMemBlockFlags_GpuCached);
    slot->chunks[slot->chunk_count++] = memory;
    dkCmdBufAddMemory(cmdbuf, memory, 0, size);
}

bool csdk_init(void) {
    if (ready) return true;
    device = siglus_gpu_dk_device();
    queue = siglus_gpu_dk_queue();
    if (!device || !queue) return false;
    const uint32_t descriptors = ImageDescriptors * sizeof(DkImageDescriptor) + SamplerDescriptors * sizeof(DkSamplerDescriptor);
    descriptor_memory = make_memory(descriptors, DkMemBlockFlags_CpuUncached | DkMemBlockFlags_GpuCached);
    image_descriptors = (DkImageDescriptor*) dkMemBlockGetCpuAddr(descriptor_memory);
    sampler_descriptors = (DkSamplerDescriptor*) (image_descriptors + ImageDescriptors);
    memset(image_descriptors, 0, descriptors);
    code_memory = make_memory(CodeSize, DkMemBlockFlags_CpuUncached | DkMemBlockFlags_GpuCached | DkMemBlockFlags_Code);
    for (unsigned i = 0; i < Slots; ++i) {
        DkCmdBufMaker maker;
        dkCmdBufMakerDefaults(&maker, device);
        maker.userData = &slots[i];
        maker.cbAddMem = add_memory;
        slots[i].cmdbuf = dkCmdBufCreate(&maker);
    }
    display_count = siglus_gpu_dk_framebuffer_count();
    for (unsigned i = 0; i < display_count && i < 8; ++i) {
        for (int32_t id = 1; id < MaxImages; ++id) {
            if (images[id].used) continue;
            Image* img = &images[id];
            memset(img, 0, sizeof(*img));
            img->used = true;
            img->owned = false;
            img->image = *siglus_gpu_dk_framebuffer(i);
            img->format = CsFmt_Rgba8Unorm;
            img->type = CsType_2D;
            img->width = csdk_display_width();
            img->height = csdk_display_height();
            img->depth = 1;
            img->mips = 1;
            img->samples = 1;
            display_ids[i] = id;
            break;
        }
    }
    ready = true;
    siglus_switch_log_message("csdk: ready\n");
    return true;
}

uint32_t csdk_display_width(void) { return 1280; }
uint32_t csdk_display_height(void) { return 720; }
int32_t csdk_display_image(uint32_t index) { return index < display_count ? display_ids[index] : -1; }

int32_t csdk_acquire(void) {
    int slot = -1;
    DkFence fence = {0};
    *(uint32_t*) fence._storage = 2; /* deko3d Fence::Status_Waiting, as gpu.c */
    NvMultiFence* nv = (NvMultiFence*) (void*) (fence._storage + sizeof(uint32_t));
    const Result rc = nwindowDequeueBuffer(siglus_gpu_dk_window(), &slot, nv);
    if (R_FAILED(rc) || slot < 0 || (unsigned) slot >= display_count) return -1;
    dkQueueWaitFence(queue, &fence);
    return slot;
}

void csdk_present(int32_t index) {
    if (index >= 0) dkQueuePresentImage(queue, siglus_gpu_dk_swapchain(), index);
}

int32_t csdk_mem_create(uint64_t size, uint32_t kind) {
    for (int32_t id = 1; id < MaxMem; ++id) {
        if (mems[id]) continue;
        uint32_t flags = kind == CsMem_Readback ? (DkMemBlockFlags_CpuCached | DkMemBlockFlags_GpuCached)
                                                 : (DkMemBlockFlags_CpuUncached | DkMemBlockFlags_GpuCached);
        mems[id] = make_memory(size ? size : 1, flags);
        if (!mems[id]) { logf_("csdk: memblock %d bytes failed (%d)\n", (int) size, 0); return -1; }
        mem_sizes[id] = (uint32_t) size;
        return id;
    }
    siglus_switch_log_message("csdk: memory table exhausted\n");
    return -1;
}
void csdk_mem_destroy(int32_t id) {
    if (id > 0 && id < MaxMem && mems[id]) { dkMemBlockDestroy(mems[id]); mems[id] = NULL; }
}
void* csdk_mem_cpu(int32_t id) { return id > 0 && id < MaxMem && mems[id] ? dkMemBlockGetCpuAddr(mems[id]) : NULL; }
uint64_t csdk_mem_gpu(int32_t id) { return id > 0 && id < MaxMem && mems[id] ? dkMemBlockGetGpuAddr(mems[id]) : 0; }
void csdk_mem_invalidate(int32_t id, uint64_t offset, uint64_t size) {
    if (id > 0 && id < MaxMem && mems[id]) dkMemBlockFlushCpuCache(mems[id], (uint32_t) offset, (uint32_t) size);
}

int32_t csdk_image_create(uint32_t format, uint32_t type, uint32_t width, uint32_t height,
                          uint32_t depth_or_layers, uint32_t mips, uint32_t samples, uint32_t flags) {
    int32_t id = -1;
    for (int32_t i = 1; i < MaxImages; ++i) if (!images[i].used) { id = i; break; }
    if (id < 0) { siglus_switch_log_message("csdk: image table exhausted\n"); return -1; }
    Image* img = &images[id];
    memset(img, 0, sizeof(*img));
    DkImageLayoutMaker maker;
    dkImageLayoutMakerDefaults(&maker, device);
    maker.type = dk_type(type);
    maker.format = dk_format(format);
    maker.msMode = ms_mode(samples);
    maker.flags = 0;
    if (flags & CsImage_Render) maker.flags |= DkImageFlags_UsageRender | DkImageFlags_HwCompression;
    /* Resolves (MSAA) and image-to-image copies run on the 2D engine. */
    if (flags & (CsImage_Render | CsImage_Copy)) maker.flags |= DkImageFlags_Usage2DEngine;
    if (flags & CsImage_Storage) maker.flags |= DkImageFlags_UsageLoadStore;
    maker.dimensions[0] = width;
    maker.dimensions[1] = height;
    maker.dimensions[2] = type == CsType_2D || type == CsType_2DMS ? 1 : depth_or_layers;
    maker.mipLevels = mips ? mips : 1;
    if (maker.format == DkImageFormat_None) { logf_("csdk: unsupported format %d (%d)\n", (int) format, 0); return -1; }
    DkImageLayout layout;
    dkImageLayoutInitialize(&layout, &maker);
    const uint32_t alignment = dkImageLayoutGetAlignment(&layout);
    const uint64_t size = dkImageLayoutGetSize(&layout);
    DkMemBlockMaker mm;
    dkMemBlockMakerDefaults(&mm, device, align_up((uint32_t) size, alignment > DK_MEMBLOCK_ALIGNMENT ? alignment : DK_MEMBLOCK_ALIGNMENT));
    mm.flags = DkMemBlockFlags_GpuCached | DkMemBlockFlags_Image;
    img->memory = dkMemBlockCreate(&mm);
    if (!img->memory) { logf_("csdk: image memory %d bytes failed (%d)\n", (int) size, 0); return -1; }
    dkImageInitialize(&img->image, &layout, img->memory, 0);
    img->used = true;
    img->owned = true;
    img->format = format;
    img->type = type;
    img->width = width;
    img->height = height;
    img->depth = depth_or_layers;
    img->mips = maker.mipLevels;
    img->samples = samples;
    return id;
}
void csdk_image_destroy(int32_t id) {
    if (id <= 0 || id >= MaxImages || !images[id].used) return;
    if (images[id].owned && images[id].memory) dkMemBlockDestroy(images[id].memory);
    images[id].used = false;
}

static bool make_view(DkImageView* out, const CsView* v) {
    if (!v || v->image <= 0 || v->image >= MaxImages || !images[v->image].used) return false;
    const Image* img = &images[v->image];
    dkImageViewDefaults(out, &img->image);
    if (v->type != img->type) out->type = dk_type(v->type);
    if (v->format && v->format != img->format) out->format = dk_format(v->format);
    out->mipLevelOffset = (uint8_t) v->mip;
    out->mipLevelCount = (uint8_t) (v->mips ? v->mips : img->mips - v->mip);
    out->layerOffset = (uint16_t) v->layer;
    out->layerCount = (uint16_t) v->layers;
    if (v->stencil) out->dsSource = DkDsSource_Stencil;
    return true;
}

bool csdk_image_descriptor(uint32_t slot, const CsView* view, bool load_store) {
    if (slot >= ImageDescriptors) return false;
    DkImageView v;
    if (!make_view(&v, view)) return false;
    dkImageDescriptorInitialize(&image_descriptors[slot], &v, load_store, false);
    return true;
}

static DkWrapMode dk_wrap(uint32_t w) {
    switch (w) {
    case CsWrap_Repeat: return DkWrapMode_Repeat;
    case CsWrap_Mirror: return DkWrapMode_MirroredRepeat;
    case CsWrap_Border: return DkWrapMode_ClampToBorder;
    default: return DkWrapMode_ClampToEdge;
    }
}
bool csdk_sampler_descriptor(uint32_t slot, const CsSampler* s) {
    if (slot >= SamplerDescriptors) return false;
    DkSampler sampler;
    dkSamplerDefaults(&sampler);
    sampler.minFilter = s->min_filter == CsFilter_Linear ? DkFilter_Linear : DkFilter_Nearest;
    sampler.magFilter = s->mag_filter == CsFilter_Linear ? DkFilter_Linear : DkFilter_Nearest;
    sampler.mipFilter = s->mip_filter == CsFilter_Linear ? DkMipFilter_Linear : DkMipFilter_Nearest;
    for (int i = 0; i < 3; ++i) sampler.wrapMode[i] = dk_wrap(s->wrap[i]);
    sampler.lodClampMin = s->lod_min;
    sampler.lodClampMax = s->lod_max;
    sampler.compareEnable = s->compare_enable != 0;
    sampler.compareOp = dk_cmp((uint8_t) s->compare);
    sampler.maxAnisotropy = s->anisotropy < 1.0f ? 1.0f : s->anisotropy;
    for (int i = 0; i < 4; ++i) sampler.borderColor[i].value_f = s->border[i];
    dkSamplerDescriptorInitialize(&sampler_descriptors[slot], &sampler);
    return true;
}

int32_t csdk_shader(const void* code, uint32_t size) {
    if (shader_count >= MaxShaders || code_used + align_up(size, DK_SHADER_CODE_ALIGNMENT) > CodeSize) {
        siglus_switch_log_message("csdk: shader storage exhausted\n");
        return -1;
    }
    const uint32_t offset = code_used;
    code_used += align_up(size, DK_SHADER_CODE_ALIGNMENT);
    memcpy((uint8_t*) dkMemBlockGetCpuAddr(code_memory) + offset, code, size);
    DkShaderMaker maker;
    dkShaderMakerDefaults(&maker, code_memory, offset);
    dkShaderInitialize(&shaders[shader_count], &maker);
    if (!dkShaderIsValid(&shaders[shader_count])) { siglus_switch_log_message("csdk: invalid DKSH\n"); return -1; }
    return (int32_t) ++shader_count;
}
static const DkShader* shader_at(int32_t id) {
    return id > 0 && (uint32_t) id <= shader_count ? &shaders[id - 1] : NULL;
}

static void poll_completed(void) {
    for (unsigned i = 0; i < Slots; ++i) {
        Slot* s = &slots[i];
        if (s->pending && dkFenceWait(&s->fence, 0) == DkResult_Success) {
            s->pending = false;
            if (s->seq > completed_seq) completed_seq = s->seq;
        }
    }
}

void csdk_begin(void) {
    Slot* slot = &slots[next_seq % Slots];
    if (slot->pending) {
        dkFenceWait(&slot->fence, -1);
        slot->pending = false;
        if (slot->seq > completed_seq) completed_seq = slot->seq;
    }
    dkCmdBufClear(slot->cmdbuf);
    for (uint32_t i = 1; i < slot->chunk_count; ++i) dkMemBlockDestroy(slot->chunks[i]);
    if (slot->chunk_count == 0) add_memory(slot, slot->cmdbuf, ChunkSize);
    else { slot->chunk_count = 1; dkCmdBufAddMemory(slot->cmdbuf, slot->chunks[0], 0, ChunkSize); }
    current = slot;
    DkCmdBuf cmd = slot->cmdbuf;
    dkCmdBufBindImageDescriptorSet(cmd, dkMemBlockGetGpuAddr(descriptor_memory), ImageDescriptors);
    dkCmdBufBindSamplerDescriptorSet(cmd, dkMemBlockGetGpuAddr(descriptor_memory) + ImageDescriptors * sizeof(DkImageDescriptor), SamplerDescriptors);
    dkCmdBufBarrier(cmd, DkBarrier_None, DkInvalidateFlags_Image | DkInvalidateFlags_Shader | DkInvalidateFlags_Descriptors | DkInvalidateFlags_L2Cache);
}

uint64_t csdk_submit(void) {
    if (!current) return completed_seq;
    DkCmdList list = dkCmdBufFinishList(current->cmdbuf);
    dkQueueSubmitCommands(queue, list);
    dkQueueSignalFence(queue, &current->fence, true);
    dkQueueFlush(queue);
    current->seq = next_seq++;
    current->pending = true;
    const uint64_t seq = current->seq;
    current = NULL;
    return seq;
}
bool csdk_done(uint64_t seq) {
    poll_completed();
    return seq <= completed_seq;
}
void csdk_wait(uint64_t seq) {
    for (unsigned i = 0; i < Slots; ++i) {
        Slot* s = &slots[i];
        if (s->pending && s->seq <= seq) {
            dkFenceWait(&s->fence, -1);
            s->pending = false;
            if (s->seq > completed_seq) completed_seq = s->seq;
        }
    }
    if (seq > completed_seq && seq < next_seq) dkQueueWaitIdle(queue), completed_seq = seq;
}

#define CMD (current->cmdbuf)

void csdk_barrier(void) {
    if (!current) return;
    dkCmdBufBarrier(CMD, DkBarrier_Full, DkInvalidateFlags_Image | DkInvalidateFlags_Shader |
                    DkInvalidateFlags_Descriptors | DkInvalidateFlags_L2Cache);
}

void csdk_targets(uint32_t count, const CsView* colors, const CsView* depth) {
    if (!current) return;
    DkImageView views[8], dview;
    const DkImageView* ptrs[8];
    for (uint32_t i = 0; i < count && i < 8; ++i) {
        make_view(&views[i], &colors[i]);
        ptrs[i] = &views[i];
    }
    const bool has_depth = depth && make_view(&dview, depth);
    dkCmdBufBindRenderTargets(CMD, ptrs, count, has_depth ? &dview : NULL);
}
void csdk_clear_color(uint32_t index, const float* c) {
    if (current) dkCmdBufClearColorFloat(CMD, index, DkColorMask_RGBA, c[0], c[1], c[2], c[3]);
}
void csdk_clear_depth_stencil(bool depth, float value, bool stencil, uint8_t stencil_value) {
    if (current) dkCmdBufClearDepthStencil(CMD, depth, value, stencil ? 0xff : 0, stencil_value);
}
void csdk_resolve(const CsView* src, const CsView* dst) {
    DkImageView a, b;
    if (current && make_view(&a, src) && make_view(&b, dst)) dkCmdBufResolveImage(CMD, &a, &b);
}
void csdk_viewport(float x, float y, float w, float h, float min_depth, float max_depth) {
    if (!current) return;
    const DkViewport v = { x, y, w, h, min_depth, max_depth };
    dkCmdBufSetViewports(CMD, 0, &v, 1);
}
void csdk_scissor(uint32_t x, uint32_t y, uint32_t w, uint32_t h) {
    if (!current) return;
    const DkScissor s = { x, y, w, h };
    dkCmdBufSetScissors(CMD, 0, &s, 1);
}
void csdk_blend_constant(const float* c) { if (current) dkCmdBufSetBlendConst(CMD, c[0], c[1], c[2], c[3]); }
void csdk_stencil_reference(uint8_t reference, uint8_t read_mask, uint8_t write_mask) {
    if (current) dkCmdBufSetStencil(CMD, DkFace_FrontAndBack, write_mask, reference, read_mask);
}

static DkVtxAttribSize attrib_size(uint32_t bits, uint32_t n) {
    if (bits == 8) return n == 1 ? DkVtxAttribSize_1x8 : n == 2 ? DkVtxAttribSize_2x8 : n == 3 ? DkVtxAttribSize_3x8 : DkVtxAttribSize_4x8;
    if (bits == 16) return n == 1 ? DkVtxAttribSize_1x16 : n == 2 ? DkVtxAttribSize_2x16 : n == 3 ? DkVtxAttribSize_3x16 : DkVtxAttribSize_4x16;
    return n == 1 ? DkVtxAttribSize_1x32 : n == 2 ? DkVtxAttribSize_2x32 : n == 3 ? DkVtxAttribSize_3x32 : DkVtxAttribSize_4x32;
}
static DkVtxAttribType attrib_type(uint32_t t) {
    switch (t) {
    case CsVt_Uint: return DkVtxAttribType_Uint;
    case CsVt_Sint: return DkVtxAttribType_Sint;
    case CsVt_Unorm: return DkVtxAttribType_Unorm;
    case CsVt_Snorm: return DkVtxAttribType_Snorm;
    default: return DkVtxAttribType_Float;
    }
}

void csdk_bind_graphics(const CsPipeline* p) {
    if (!current) return;
    const DkShader* list[2];
    uint32_t n = 0;
    if (shader_at(p->vs)) list[n++] = shader_at(p->vs);
    if (shader_at(p->fs)) list[n++] = shader_at(p->fs);
    dkCmdBufBindShaders(CMD, DkStageFlag_GraphicsMask, list, n);

    DkRasterizerState r;
    dkRasterizerStateDefaults(&r);
    r.cullMode = p->cull == CsCull_Front ? DkFace_Front : p->cull == CsCull_Back ? DkFace_Back : DkFace_None;
    r.frontFace = p->front_face == CsFront_Cw ? DkFrontFace_CW : DkFrontFace_CCW;
    r.depthClampEnable = p->unclipped_depth != 0;
    r.depthBiasEnableMask = (p->depth_bias != 0 || p->depth_bias_slope != 0.0f) ? 0x7 : 0;
    dkCmdBufBindRasterizerState(CMD, &r);
    if (r.depthBiasEnableMask) dkCmdBufSetDepthBias(CMD, (float) p->depth_bias, p->depth_bias_clamp, p->depth_bias_slope);

    DkColorState cs;
    dkColorStateDefaults(&cs);
    DkColorWriteState cw;
    dkColorWriteStateDefaults(&cw);
    uint32_t blend_mask = 0;
    for (uint32_t i = 0; i < p->color_count && i < 8; ++i) {
        const CsBlend* b = &p->blends[i];
        dkColorWriteStateSetMask(&cw, i, b->write_mask);
        if (!b->enable) continue;
        blend_mask |= 1u << i;
        DkBlendState bs;
        dkBlendStateDefaults(&bs);
        bs.colorBlendOp = dk_op(b->color_op);
        bs.srcColorBlendFactor = dk_factor(b->color_src);
        bs.dstColorBlendFactor = dk_factor(b->color_dst);
        bs.alphaBlendOp = dk_op(b->alpha_op);
        bs.srcAlphaBlendFactor = dk_factor(b->alpha_src);
        bs.dstAlphaBlendFactor = dk_factor(b->alpha_dst);
        dkCmdBufBindBlendState(CMD, i, &bs);
    }
    cs.blendEnableMask = blend_mask;
    dkCmdBufBindColorState(CMD, &cs);
    dkCmdBufBindColorWriteState(CMD, &cw);

    DkDepthStencilState ds;
    dkDepthStencilStateDefaults(&ds);
    ds.depthTestEnable = p->depth_test;
    ds.depthWriteEnable = p->depth_write;
    ds.depthCompareOp = dk_cmp(p->depth_compare);
    ds.stencilTestEnable = p->stencil_enable;
    ds.stencilFrontCompareOp = dk_cmp(p->stencil_front[0]);
    ds.stencilFrontFailOp = dk_sop(p->stencil_front[1]);
    ds.stencilFrontDepthFailOp = dk_sop(p->stencil_front[2]);
    ds.stencilFrontPassOp = dk_sop(p->stencil_front[3]);
    ds.stencilBackCompareOp = dk_cmp(p->stencil_back[0]);
    ds.stencilBackFailOp = dk_sop(p->stencil_back[1]);
    ds.stencilBackDepthFailOp = dk_sop(p->stencil_back[2]);
    ds.stencilBackPassOp = dk_sop(p->stencil_back[3]);
    dkCmdBufBindDepthStencilState(CMD, &ds);

    DkMultisampleState ms;
    dkMultisampleStateDefaults(&ms);
    ms.mode = ms_mode(p->samples);
    ms.rasterizerMode = ms_mode(p->samples);
    ms.alphaToCoverageEnable = p->alpha_to_coverage;
    dkCmdBufBindMultisampleState(CMD, &ms);

    DkVtxAttribState attribs[32];
    uint32_t count = 0;
    for (uint32_t i = 0; i < p->attrib_count; ++i) if (p->attribs[i].location + 1 > count) count = p->attribs[i].location + 1;
    if (count > 32) count = 32;
    for (uint32_t i = 0; i < count; ++i) {
        memset(&attribs[i], 0, sizeof(attribs[i]));
        attribs[i].isFixed = 1;
        attribs[i].size = DkVtxAttribSize_1x32;
        attribs[i].type = DkVtxAttribType_Float;
    }
    for (uint32_t i = 0; i < p->attrib_count; ++i) {
        const CsAttrib* a = &p->attribs[i];
        if (a->location >= count) continue;
        DkVtxAttribState* s = &attribs[a->location];
        s->isFixed = 0;
        s->bufferId = a->buffer;
        s->offset = a->offset;
        s->size = attrib_size(a->bits, a->components);
        s->type = a->type == CsVt_Half ? DkVtxAttribType_Float : attrib_type(a->type);
    }
    dkCmdBufBindVtxAttribState(CMD, attribs, count);
    DkVtxBufferState buffers[16];
    for (uint32_t i = 0; i < p->buffer_count && i < 16; ++i) {
        buffers[i].stride = p->strides[i];
        buffers[i].divisor = p->instanced[i] ? 1 : 0;
    }
    dkCmdBufBindVtxBufferState(CMD, buffers, p->buffer_count);
}

void csdk_bind_compute(int32_t cs) {
    const DkShader* s = shader_at(cs);
    if (current && s) dkCmdBufBindShaders(CMD, DkStageFlag_Compute, &s, 1);
}
void csdk_vertex_buffer(uint32_t slot, uint64_t gpu, uint32_t size) { if (current) dkCmdBufBindVtxBuffer(CMD, slot, gpu, size); }
void csdk_index_buffer(uint64_t gpu, uint32_t format) {
    if (current) dkCmdBufBindIdxBuffer(CMD, format == CsIdx_U16 ? DkIdxFormat_Uint16 : DkIdxFormat_Uint32, gpu);
}
void csdk_uniform(uint32_t stage, uint32_t slot, uint64_t gpu, uint32_t size) {
    if (current) dkCmdBufBindUniformBuffer(CMD, dk_stage(stage), slot, gpu, align_up(size, 16));
}
void csdk_storage(uint32_t stage, uint32_t slot, uint64_t gpu, uint32_t size) {
    if (current) dkCmdBufBindStorageBuffer(CMD, dk_stage(stage), slot, gpu, size);
}
void csdk_texture(uint32_t stage, uint32_t slot, uint32_t image_slot, uint32_t sampler_slot) {
    if (current) dkCmdBufBindTexture(CMD, dk_stage(stage), slot, dkMakeTextureHandle(image_slot, sampler_slot));
}
void csdk_image(uint32_t stage, uint32_t slot, uint32_t image_slot) {
    if (current) dkCmdBufBindImage(CMD, dk_stage(stage), slot, dkMakeImageHandle(image_slot));
}
void csdk_draw(uint32_t prim, uint32_t count, uint32_t instances, uint32_t first, uint32_t first_instance) {
    if (current) dkCmdBufDraw(CMD, dk_prim(prim), count, instances, first, first_instance);
}
void csdk_draw_indexed(uint32_t prim, uint32_t count, uint32_t instances, uint32_t first, int32_t base_vertex, uint32_t first_instance) {
    if (current) dkCmdBufDrawIndexed(CMD, dk_prim(prim), count, instances, first, base_vertex, first_instance);
}
void csdk_draw_indirect(uint32_t prim, uint64_t gpu, bool indexed) {
    if (!current) return;
    if (indexed) dkCmdBufDrawIndexedIndirect(CMD, dk_prim(prim), gpu);
    else dkCmdBufDrawIndirect(CMD, dk_prim(prim), gpu);
}
void csdk_dispatch(uint32_t x, uint32_t y, uint32_t z) { if (current) dkCmdBufDispatchCompute(CMD, x, y, z); }
void csdk_dispatch_indirect(uint64_t gpu) { if (current) dkCmdBufDispatchComputeIndirect(CMD, gpu); }
void csdk_copy_buffer(uint64_t src, uint64_t dst, uint64_t size) {
    if (current && size) dkCmdBufCopyBuffer(CMD, src, dst, (uint32_t) size);
}
void csdk_copy_buffer_to_image(uint64_t src, uint32_t row_bytes, uint32_t image_rows, const CsView* dst,
                               uint32_t x, uint32_t y, uint32_t z, uint32_t w, uint32_t h, uint32_t d) {
    DkImageView v;
    if (!current || !make_view(&v, dst)) return;
    const DkCopyBuf buf = { src, row_bytes, image_rows };
    const DkImageRect rect = { x, y, z, w, h, d };
    dkCmdBufCopyBufferToImage(CMD, &buf, &v, &rect, 0);
}
void csdk_copy_image_to_buffer(const CsView* src, uint32_t x, uint32_t y, uint32_t z, uint32_t w, uint32_t h,
                               uint32_t d, uint64_t dst, uint32_t row_pixels, uint32_t image_rows) {
    DkImageView v;
    if (!current || !make_view(&v, src)) return;
    const DkCopyBuf buf = { dst, row_pixels, image_rows };
    const DkImageRect rect = { x, y, z, w, h, d };
    dkCmdBufCopyImageToBuffer(CMD, &v, &rect, &buf, 0);
}
void csdk_copy_image(const CsView* src, uint32_t sx, uint32_t sy, uint32_t sz, const CsView* dst,
                     uint32_t dx, uint32_t dy, uint32_t dz, uint32_t w, uint32_t h, uint32_t d) {
    DkImageView a, b;
    if (!current || !make_view(&a, src) || !make_view(&b, dst)) return;
    const DkImageRect ra = { sx, sy, sz, w, h, d }, rb = { dx, dy, dz, w, h, d };
    dkCmdBufCopyImage(CMD, &a, &ra, &b, &rb, 0);
}
