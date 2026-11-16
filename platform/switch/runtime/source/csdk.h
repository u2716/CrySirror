/* SPDX-License-Identifier: MPL-2.0
 * CrySirror deko3d driver ABI for the wgpu custom backend
 * (crates/crysirror-wgpu-deko3d/src/driver.rs mirrors every struct and
 * constant). All deko3d enum values are mapped here, never in Rust. */
#pragma once
#include <stdbool.h>
#include <stdint.h>

/* Formats */
enum {
    CsFmt_None, CsFmt_R8Unorm, CsFmt_R8Snorm, CsFmt_R8Uint, CsFmt_R8Sint,
    CsFmt_R16Float, CsFmt_R16Unorm, CsFmt_R16Uint, CsFmt_R16Sint,
    CsFmt_R32Float, CsFmt_R32Uint, CsFmt_R32Sint,
    CsFmt_Rg8Unorm, CsFmt_Rg8Snorm, CsFmt_Rg8Uint, CsFmt_Rg8Sint,
    CsFmt_Rg16Float, CsFmt_Rg16Unorm, CsFmt_Rg16Uint, CsFmt_Rg16Sint,
    CsFmt_Rg32Float, CsFmt_Rg32Uint, CsFmt_Rg32Sint,
    CsFmt_Rgba8Unorm, CsFmt_Rgba8UnormSrgb, CsFmt_Rgba8Snorm, CsFmt_Rgba8Uint, CsFmt_Rgba8Sint,
    CsFmt_Bgra8Unorm, CsFmt_Bgra8UnormSrgb,
    CsFmt_Rgba16Float, CsFmt_Rgba16Unorm, CsFmt_Rgba16Uint, CsFmt_Rgba16Sint,
    CsFmt_Rgba32Float, CsFmt_Rgba32Uint, CsFmt_Rgba32Sint,
    CsFmt_Rgb10a2Unorm, CsFmt_Rg11b10Float,
    CsFmt_Depth16, CsFmt_Depth24, CsFmt_Depth24Stencil8, CsFmt_Depth32F, CsFmt_Depth32FStencil8, CsFmt_Stencil8,
    CsFmt_Bc1, CsFmt_Bc1Srgb, CsFmt_Bc2, CsFmt_Bc2Srgb, CsFmt_Bc3, CsFmt_Bc3Srgb,
    CsFmt_Bc4Unorm, CsFmt_Bc4Snorm, CsFmt_Bc5Unorm, CsFmt_Bc5Snorm, CsFmt_Bc6hUfloat, CsFmt_Bc6hSfloat,
    CsFmt_Bc7, CsFmt_Bc7Srgb,
    CsFmt_Count
};
/* Image/view types */
enum { CsType_2D, CsType_2DArray, CsType_Cube, CsType_CubeArray, CsType_3D, CsType_2DMS, CsType_2DMSArray, CsType_1D };
/* Image flags */
enum { CsImage_Render = 1, CsImage_Storage = 2, CsImage_Present = 4, CsImage_Copy = 8 };
/* Memory kinds */
enum { CsMem_Upload = 0, CsMem_Readback = 1, CsMem_Code = 2 };
/* Stages */
enum { CsStage_Vertex = 0, CsStage_Fragment = 1, CsStage_Compute = 2 };
/* Blend factors / ops (wgpu names) */
enum { CsBf_Zero, CsBf_One, CsBf_Src, CsBf_OneMinusSrc, CsBf_SrcAlpha, CsBf_OneMinusSrcAlpha, CsBf_Dst,
       CsBf_OneMinusDst, CsBf_DstAlpha, CsBf_OneMinusDstAlpha, CsBf_SrcAlphaSaturated, CsBf_Constant,
       CsBf_OneMinusConstant };
enum { CsBo_Add, CsBo_Subtract, CsBo_ReverseSubtract, CsBo_Min, CsBo_Max };
enum { CsCmp_Never, CsCmp_Less, CsCmp_Equal, CsCmp_LessEqual, CsCmp_Greater, CsCmp_NotEqual, CsCmp_GreaterEqual, CsCmp_Always };
enum { CsSop_Keep, CsSop_Zero, CsSop_Replace, CsSop_Invert, CsSop_IncrementClamp, CsSop_DecrementClamp, CsSop_IncrementWrap, CsSop_DecrementWrap };
enum { CsCull_None, CsCull_Front, CsCull_Back };
enum { CsFront_Ccw, CsFront_Cw };
enum { CsPrim_Points, CsPrim_Lines, CsPrim_LineStrip, CsPrim_Triangles, CsPrim_TriangleStrip };
enum { CsIdx_U16, CsIdx_U32 };
enum { CsFilter_Nearest, CsFilter_Linear };
enum { CsWrap_Clamp, CsWrap_Repeat, CsWrap_Mirror, CsWrap_Border };
/* Vertex formats: component type and count. */
enum { CsVt_Float, CsVt_Uint, CsVt_Sint, CsVt_Unorm, CsVt_Snorm, CsVt_Half };

typedef struct CsView {
    int32_t image;
    uint32_t type;       /* CsType_* */
    uint32_t format;     /* CsFmt_*; CsFmt_None keeps the image format */
    uint32_t mip, mips;
    uint32_t layer, layers;
    uint32_t stencil;    /* 1: sample the stencil aspect */
} CsView;

typedef struct CsBlend {
    uint8_t enable, color_op, color_src, color_dst, alpha_op, alpha_src, alpha_dst, write_mask;
} CsBlend;

typedef struct CsAttrib {
    uint32_t location, buffer, offset;
    uint32_t type;       /* CsVt_* */
    uint32_t bits;       /* 8, 16, 32 */
    uint32_t components; /* 1..4 */
} CsAttrib;

typedef struct CsPipeline {
    int32_t vs, fs;
    uint32_t cull, front_face, unclipped_depth;
    uint32_t color_count;
    CsBlend blends[8];
    uint8_t depth_test, depth_write, depth_compare, stencil_enable;
    uint8_t stencil_front[4], stencil_back[4]; /* compare, fail, depth_fail, pass */
    uint8_t stencil_read_mask, stencil_write_mask, alpha_to_coverage, pad;
    int32_t depth_bias;
    float depth_bias_slope, depth_bias_clamp;
    uint32_t samples;
    uint32_t attrib_count;
    CsAttrib attribs[16];
    uint32_t buffer_count;
    uint32_t strides[16];
    uint32_t instanced[16];
} CsPipeline;

typedef struct CsSampler {
    uint32_t min_filter, mag_filter, mip_filter;
    uint32_t wrap[3];
    float lod_min, lod_max;
    uint32_t compare_enable, compare;
    float anisotropy;
    float border[4];
} CsSampler;

/* Device (shares gpu.c's device, queue and swapchain) */
bool csdk_init(void);
uint32_t csdk_display_width(void);
uint32_t csdk_display_height(void);
int32_t csdk_display_image(uint32_t index);   /* registered framebuffer image ids */
int32_t csdk_acquire(void);                    /* framebuffer index or -1 */
void csdk_present(int32_t index);

/* Memory */
int32_t csdk_mem_create(uint64_t size, uint32_t kind);
void csdk_mem_destroy(int32_t id);
void* csdk_mem_cpu(int32_t id);
uint64_t csdk_mem_gpu(int32_t id);
void csdk_mem_invalidate(int32_t id, uint64_t offset, uint64_t size);

/* Images, descriptors, shaders */
int32_t csdk_image_create(uint32_t format, uint32_t type, uint32_t width, uint32_t height,
                          uint32_t depth_or_layers, uint32_t mips, uint32_t samples, uint32_t flags);
void csdk_image_destroy(int32_t id);
bool csdk_image_descriptor(uint32_t slot, const CsView* view, bool load_store);
bool csdk_sampler_descriptor(uint32_t slot, const CsSampler* sampler);
int32_t csdk_shader(const void* code, uint32_t size);

/* Recording: one command list per submission */
void csdk_begin(void);
uint64_t csdk_submit(void);          /* returns the submission's fence sequence */
bool csdk_done(uint64_t seq);
void csdk_wait(uint64_t seq);
void csdk_barrier(void);
void csdk_targets(uint32_t count, const CsView* colors, const CsView* depth);
void csdk_clear_color(uint32_t index, const float* rgba);
void csdk_clear_depth_stencil(bool depth, float value, bool stencil, uint8_t stencil_value);
void csdk_resolve(const CsView* src, const CsView* dst);
void csdk_viewport(float x, float y, float w, float h, float min_depth, float max_depth);
void csdk_scissor(uint32_t x, uint32_t y, uint32_t w, uint32_t h);
void csdk_blend_constant(const float* rgba);
void csdk_stencil_reference(uint8_t reference, uint8_t read_mask, uint8_t write_mask);
void csdk_bind_graphics(const CsPipeline* pipeline);
void csdk_bind_compute(int32_t cs);
void csdk_vertex_buffer(uint32_t slot, uint64_t gpu, uint32_t size);
void csdk_index_buffer(uint64_t gpu, uint32_t format);
void csdk_uniform(uint32_t stage, uint32_t slot, uint64_t gpu, uint32_t size);
void csdk_storage(uint32_t stage, uint32_t slot, uint64_t gpu, uint32_t size);
void csdk_texture(uint32_t stage, uint32_t slot, uint32_t image_slot, uint32_t sampler_slot);
void csdk_image(uint32_t stage, uint32_t slot, uint32_t image_slot);
void csdk_draw(uint32_t prim, uint32_t count, uint32_t instances, uint32_t first, uint32_t first_instance);
void csdk_draw_indexed(uint32_t prim, uint32_t count, uint32_t instances, uint32_t first, int32_t base_vertex, uint32_t first_instance);
void csdk_draw_indirect(uint32_t prim, uint64_t gpu, bool indexed);
void csdk_dispatch(uint32_t x, uint32_t y, uint32_t z);
void csdk_dispatch_indirect(uint64_t gpu);
void csdk_copy_buffer(uint64_t src, uint64_t dst, uint64_t size);
void csdk_copy_buffer_to_image(uint64_t src, uint32_t row_pixels, uint32_t image_rows, const CsView* dst,
                               uint32_t x, uint32_t y, uint32_t z, uint32_t w, uint32_t h, uint32_t d);
void csdk_copy_image_to_buffer(const CsView* src, uint32_t x, uint32_t y, uint32_t z, uint32_t w, uint32_t h,
                               uint32_t d, uint64_t dst, uint32_t row_pixels, uint32_t image_rows);
void csdk_copy_image(const CsView* src, uint32_t sx, uint32_t sy, uint32_t sz, const CsView* dst,
                     uint32_t dx, uint32_t dy, uint32_t dz, uint32_t w, uint32_t h, uint32_t d);
