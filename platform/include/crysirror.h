/* SPDX-License-Identifier: MPL-2.0 */
#ifndef CRYSIRROR_H
#define CRYSIRROR_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* UTF-8 requests; returned JSON is owned and must be freed exactly once.
 * start: {game:absolutePath,state_dir:absolutePath,nls:"auto"|nativeOption,muted:bool}
 * view: CAMetalLayer* on Apple, ANativeWindow* on Android. Caller must retain
 * it until crysirror_detach(session,generation) or crysirror_destroy returns. No engine object may cross this ABI.
 * start returns {session:id,surface_generation:1}; attach returns a new generation.
 * Detach keeps the VM/device/session, retires only that generation and waits
 * until its Surface is no longer in use. Late detach cannot remove a newer view.
 * Status includes phase,message,frames,attached,surface_generation.
 * command: status, resize(width,height physical pixels), input(x,y physical,
 * down CrySirror button mask), pause(paused), close (native quit policy).
 * destroy forcibly stops/joins; called only after view callbacks are detached.
 */
char *crysirror_probe(const char *path);
char *crysirror_start(const char *request, void *view, uint32_t width, uint32_t height);
char *crysirror_attach(uint64_t session, void *view, uint32_t width, uint32_t height);
char *crysirror_detach(uint64_t session, uint64_t generation);
char *crysirror_command(uint64_t session, const char *request);
void crysirror_destroy(uint64_t session);
void crysirror_string_free(char *string);
#ifdef __cplusplus
}
#endif
#endif
