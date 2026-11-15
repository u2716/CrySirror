/* SPDX-License-Identifier: MPL-2.0
 * CrySirror Switch shell. libnx owns the process, display (deko3d, gpu.c),
 * input sampling and the audio device; the Rust console player owns game
 * selection and the engine session (crysirror_console_* ABI).
 * Derived from xmoezzz/siglus_rs platform/switch/runtime/source/main.c at
 * 19066ea7f36b92fdecaea240a223c9bef14efd2c (MPL-2.0). */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <switch.h>
#include <switch/nvidia/address_space.h>
#include <switch/nvidia/map.h>

#include "gpu.h"

#define CRYSIRROR_ROOT "sdmc:/switch/CrySirror"
#define CRYSIRROR_LOG CRYSIRROR_ROOT "/crysirror.log"

enum {
    AudioBufferFrames = 960,
    AudioBufferCount = 6,
    AudioBufferStorageFrames = 1024,
};
static AudioDriver audio_driver;
static AudioDriverWaveBuf audio_wavebufs[AudioBufferCount];
static int16_t audio_buffers[AudioBufferCount][AudioBufferStorageFrames * 2]
    __attribute__((aligned(0x1000)));
static int16_t silent_buffer[AudioBufferFrames * 2];
static bool audio_renderer_initialized;
static bool audio_driver_initialized;

static void append_log(const char* message);
static void log_result(const char* operation, Result result);
extern void* crysirror_console_open(const char* root, uint32_t width, uint32_t height);
extern bool crysirror_console_step(void* player, uint32_t dt_ms);
extern void crysirror_console_button(void* player, uint8_t button, bool down);
extern void crysirror_console_touch(void* player, int32_t phase, double x, double y);
extern void crysirror_console_render_pcm(void* player, int16_t* dst, size_t frames);
extern void crysirror_console_close(void* player);
extern const char* crysirror_console_last_error(void);
extern void crysirror_console_set_system_font(const void* data, size_t size);

/* The system shared font (Standard: Japanese/Latin UD gothic) stays in the
 * pl service's read-only shared memory; it is never written to storage. */
static void register_system_font(void) {
    Result rc = plInitialize(PlServiceType_User);
    if (R_FAILED(rc)) { log_result("plInitialize", rc); return; }
    PlFontData font;
    rc = plGetSharedFontByType(&font, PlSharedFontType_Standard);
    if (R_FAILED(rc)) { log_result("plGetSharedFontByType", rc); return; }
    crysirror_console_set_system_font(font.address, font.size);
}

static void append_log(const char* message) {
    FILE* const file = fopen(CRYSIRROR_LOG, "ab");
    if (file == NULL) return;
    fputs(message, file);
    fclose(file);
}
/* Rust console player and the integrated runtimes log through these. */
void crysirror_switch_log(const char* message) { append_log(message); }
void siglus_switch_log_message(const char* message) { append_log(message); }

static void show_error(const char* message) {
    const char* details = "CrySirror could not continue.\n\nLog: " CRYSIRROR_LOG;
    Result rc;
    if (appletGetAppletType() == AppletType_Application) {
        ErrorApplicationConfig config;
        rc = errorApplicationCreate(&config, message, details);
        if (R_SUCCEEDED(rc)) rc = errorApplicationShow(&config);
    } else {
        ErrorSystemConfig config;
        rc = errorSystemCreate(&config, message, details);
        if (R_SUCCEEDED(rc)) rc = errorSystemShow(&config);
    }
    if (R_FAILED(rc)) fprintf(stderr, "error dialog 0x%08x: %s\n", rc, message);
}

static void log_result(const char* operation, Result result) {
    char message[96];
    snprintf(message, sizeof(message), "crysirror: %s rc=0x%08x\n", operation, (unsigned int) result);
    append_log(message);
}

/* Keep the Horizon Result that deko3d compresses to DkResult_Fail. */
extern Result __real_nvMapCreate(NvMap* map, void* cpu_address, u32 size, u32 alignment, NvKind kind, bool cached);
extern Result __real_nvAddressSpaceMap(NvAddressSpace* address_space, u32 handle, bool cached, NvKind kind, iova_t* output);
Result __wrap_nvMapCreate(NvMap* map, void* cpu_address, u32 size, u32 alignment, NvKind kind, bool cached) {
    Result rc = __real_nvMapCreate(map, cpu_address, size, alignment, kind, cached);
    if (R_FAILED(rc)) log_result("nvMapCreate", rc);
    return rc;
}
Result __wrap_nvAddressSpaceMap(NvAddressSpace* address_space, u32 handle, bool cached, NvKind kind, iova_t* output) {
    Result rc = __real_nvAddressSpaceMap(address_space, handle, cached, kind, output);
    if (R_FAILED(rc)) log_result("nvAddressSpaceMap", rc);
    return rc;
}

void siglus_switch_random_fill(void* buffer, size_t length) { randomGet(buffer, length); }
/* newlib std hooks with no Horizon implementation. */
long getrandom(void* buffer, size_t length, unsigned int flags) {
    (void) flags;
    randomGet(buffer, length);
    return (long) length;
}
/* newlib's sysconf names (sys/unistd.h). Applications get three CPU cores;
 * answering 4096 for every name (the old stub) made std report 4096 CPUs, so
 * sharded maps and worker pools sized themselves for 4096 cores. */
long sysconf(int name) {
    switch (name) {
    case 2: return 100;    /* _SC_CLK_TCK */
    case 4: return 256;    /* _SC_OPEN_MAX */
    case 8: return 4096;   /* _SC_PAGESIZE */
    case 9:                /* _SC_NPROCESSORS_CONF */
    case 10: return 3;     /* _SC_NPROCESSORS_ONLN */
    case 51: return 1024;  /* _SC_GETPW_R_SIZE_MAX */
    default: return -1;
    }
}

/* Rust's ELF TLS (has-thread-local in platform/switch/rust/aarch64-switch.json)
 * reads the thread pointer from TPIDR_EL0; libnx keeps it at
 * *(TPIDRRO_EL0 + 0x1F8) (__aarch64_read_tp). Every thread publishes it in
 * TPIDR_EL0, which Horizon saves per thread, before running any Rust code.
 * (OS-key TLS exhausted libnx's few TLS slots and hung the JS host.) */
extern void* __aarch64_read_tp(void);
static inline void publish_thread_pointer(void) {
    void* tp = __aarch64_read_tp();
    __asm__ volatile("msr tpidr_el0, %0" :: "r"(tp));
}
#include <pthread.h>
typedef struct ThreadStart { void* (*start)(void*); void* arg; } ThreadStart;
static void* thread_trampoline(void* raw) {
    ThreadStart start = *(ThreadStart*) raw;
    free(raw);
    publish_thread_pointer();
    return start.start(start.arg);
}
extern int __real_pthread_create(pthread_t* thread, const pthread_attr_t* attr, void* (*start)(void*), void* arg);
int __wrap_pthread_create(pthread_t* thread, const pthread_attr_t* attr, void* (*start)(void*), void* arg) {
    ThreadStart* boxed = (ThreadStart*) malloc(sizeof(ThreadStart));
    if (boxed == NULL) return 12; /* ENOMEM */
    boxed->start = start;
    boxed->arg = arg;
    const int rc = __real_pthread_create(thread, attr, thread_trampoline, boxed);
    if (rc != 0) free(boxed);
    return rc;
}

static bool file_exists(const char* path) {
    FILE* f = fopen(path, "rb");
    if (f == NULL) return false;
    fclose(f);
    return true;
}

static void initialize_audio(void) {
    static const AudioRendererConfig config = {
        .output_rate = AudioRendererOutputRate_48kHz,
        .num_voices = 2, .num_effects = 0, .num_sinks = 1, .num_mix_objs = 1, .num_mix_buffers = 2,
    };
    Result rc = audrenInitialize(&config);
    if (R_FAILED(rc)) { log_result("audrenInitialize", rc); return; }
    audio_renderer_initialized = true;
    rc = audrvCreate(&audio_driver, &config, 2);
    if (R_FAILED(rc)) { log_result("audrvCreate", rc); return; }
    audio_driver_initialized = true;
    armDCacheFlush(audio_buffers, sizeof(audio_buffers));
    const int pool = audrvMemPoolAdd(&audio_driver, audio_buffers, sizeof(audio_buffers));
    if (pool < 0 || !audrvMemPoolAttach(&audio_driver, pool)) { append_log("crysirror: audio pool failed\n"); audio_driver_initialized = false; return; }
    static const u8 sink_channels[] = { 0, 1 };
    audrvDeviceSinkAdd(&audio_driver, AUDREN_DEFAULT_DEVICE_NAME, 2, sink_channels);
    if (R_FAILED(rc = audrvUpdate(&audio_driver))) { log_result("audrvUpdate(pool)", rc); return; }
    if (R_FAILED(rc = audrenStartAudioRenderer())) { log_result("audrenStartAudioRenderer", rc); return; }
    if (!audrvVoiceInit(&audio_driver, 0, 2, PcmFormat_Int16, 48000)) { append_log("crysirror: audrvVoiceInit failed\n"); return; }
    audrvVoiceSetDestinationMix(&audio_driver, 0, AUDREN_FINAL_MIX_ID);
    audrvVoiceSetMixFactor(&audio_driver, 0, 1.0f, 0, 0);
    audrvVoiceSetMixFactor(&audio_driver, 0, 1.0f, 1, 1);
    audrvVoiceStart(&audio_driver, 0);
    audrvUpdate(&audio_driver);
    append_log("crysirror: audio ready\n");
}

static void pump_audio(void* player) {
    if (!audio_driver_initialized) {
        /* No device (or disabled): the mixer still advances at real time. */
        crysirror_console_render_pcm(player, silent_buffer, 800);
        return;
    }
    for (unsigned i = 0; i < AudioBufferCount; ++i) {
        AudioDriverWaveBuf* wavebuf = &audio_wavebufs[i];
        if (wavebuf->state != AudioDriverWaveBufState_Free && wavebuf->state != AudioDriverWaveBufState_Done) continue;
        crysirror_console_render_pcm(player, audio_buffers[i], AudioBufferFrames);
        armDCacheFlush(audio_buffers[i], sizeof(audio_buffers[i]));
        wavebuf->data_raw = audio_buffers[i];
        wavebuf->size = (size_t) AudioBufferFrames * 2 * sizeof(int16_t);
        wavebuf->start_sample_offset = 0;
        wavebuf->end_sample_offset = AudioBufferFrames;
        wavebuf->is_looping = false;
        audrvVoiceAddWaveBuf(&audio_driver, 0, wavebuf);
    }
    audrvUpdate(&audio_driver);
}

static void exit_audio(void) {
    if (audio_driver_initialized) audrvClose(&audio_driver);
    if (audio_renderer_initialized) audrenExit();
}

int main(void) {
    publish_thread_pointer();
    Result rc = romfsMountSelf("romfs");
    log_result("romfsMountSelf", rc);
    if (R_FAILED(rc)) { show_error("Could not mount the NRO resources. Copy crysirror.nro again."); return 1; }

    siglus_gpu_init();
    if (file_exists(CRYSIRROR_ROOT "/disable-audio")) append_log("crysirror: audio disabled by marker\n");
    else initialize_audio();

    register_system_font();
    append_log("crysirror: open begin\n");
    void* player = crysirror_console_open(CRYSIRROR_ROOT, siglus_gpu_display_width(), siglus_gpu_display_height());
    if (player == NULL) {
        char message[1100];
        snprintf(message, sizeof(message), "%s", crysirror_console_last_error());
        exit_audio();
        siglus_gpu_exit();
        show_error(message[0] ? message : "Could not open a game under " CRYSIRROR_ROOT "/games.");
        romfsExit();
        return 1;
    }

    padConfigureInput(1, HidNpadStyleSet_NpadStandard);
    PadState pad;
    padInitializeDefault(&pad);
    hidInitializeTouchScreen();
    bool touch_active = false, failed = false;
    double last_x = 0.0, last_y = 0.0;
    unsigned frame_count = 0;
    uint64_t period_start = armGetSystemTick();
    append_log("crysirror: main loop\n");
    while (appletMainLoop()) {
        padUpdate(&pad);
        const uint64_t down = padGetButtonsDown(&pad), up = padGetButtonsUp(&pad);
        for (uint8_t button = 0; button < 32; ++button) {
            const uint64_t bit = UINT64_C(1) << button;
            if (down & bit) crysirror_console_button(player, button, true);
            if (up & bit) crysirror_console_button(player, button, false);
        }
        HidTouchScreenState touch_state;
        const bool has_touch = hidGetTouchScreenStates(&touch_state, 1) != 0 && touch_state.count > 0;
        if (has_touch) {
            last_x = (double) touch_state.touches[0].x;
            last_y = (double) touch_state.touches[0].y;
            crysirror_console_touch(player, touch_active ? 1 : 0, last_x, last_y);
        } else if (touch_active) {
            crysirror_console_touch(player, 2, last_x, last_y);
        }
        touch_active = has_touch;
        if (crysirror_console_step(player, 16)) {
            failed = crysirror_console_last_error()[0] != 0;
            break;
        }
        pump_audio(player);
        if (++frame_count % 600 == 0) {
            const uint64_t now = armGetSystemTick();
            char message[96];
            snprintf(message, sizeof(message), "crysirror: frame %u, %.1f fps\n", frame_count,
                     600.0 * armGetSystemTickFreq() / (double) (now - period_start));
            append_log(message);
            period_start = now;
        }
    }
    char message[1100];
    snprintf(message, sizeof(message), "%s", crysirror_console_last_error());
    crysirror_console_close(player);
    exit_audio();
    siglus_gpu_exit();
    if (failed) show_error(message);
    plExit();
    romfsExit();
    return failed ? 1 : 0;
}
