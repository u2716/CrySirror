#include "cpu_memory.h"
#include <psp2/kernel/sysmem.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

// Together with the 48 MiB newlib arena this preserves the 336 MiB CPU cap.
#define CPU_LARGE_LIMIT (288u * 1024u * 1024u)
#define PAGE 4096u
static atomic_uint reserved, peak;
typedef struct { SceUID uid; unsigned mapped; } Allocation;

size_t sena_vita_cpu_reserved(void) { return atomic_load(&reserved); }
size_t sena_vita_cpu_peak(void) { return atomic_load(&peak); }

void *sena_vita_cpu_alloc(size_t bytes, bool zero) {
  if (bytes > CPU_LARGE_LIMIT - 2 * PAGE)
    return NULL;
  unsigned mapped = (bytes + 2 * PAGE - 1) & ~(PAGE - 1);
  unsigned current = atomic_load(&reserved);
  do {
    if (mapped > CPU_LARGE_LIMIT - current) {
      fprintf(stderr, "[cpu-budget] requested=%u mapped=%u live=%u limit=%u\n",
              (unsigned)bytes, mapped, current, CPU_LARGE_LIMIT);
      return NULL;
    }
  } while (!atomic_compare_exchange_weak(&reserved, &current, current + mapped));
  SceUID uid = sceKernelAllocMemBlock("sena-cpu", SCE_KERNEL_MEMBLOCK_TYPE_USER_RW,
                                    mapped, NULL);
  void *base = NULL;
  int result = uid < 0 ? uid : sceKernelGetMemBlockBase(uid, &base);
  if (result < 0) {
    if (uid >= 0) sceKernelFreeMemBlock(uid);
    atomic_fetch_sub(&reserved, mapped);
    fprintf(stderr, "[cpu-block-failed] requested=%u mapped=%u result=%x\n",
            (unsigned)bytes, mapped, result);
    return NULL;
  }
  *(Allocation *)base = (Allocation){uid, mapped};
  void *data = (uint8_t *)base + PAGE;
  if (zero) memset(data, 0, bytes);
  current += mapped;
  unsigned high = atomic_load(&peak);
  while (current > high && !atomic_compare_exchange_weak(&peak, &high, current)) {}
  return data;
}

void sena_vita_cpu_free(void *ptr) {
  Allocation allocation = *(Allocation *)((uint8_t *)ptr - PAGE);
  int result = sceKernelFreeMemBlock(allocation.uid);
  if (result < 0) {
    fprintf(stderr, "[cpu-block-free-failed] uid=%x result=%x\n", allocation.uid, result);
    return;
  }
  atomic_fetch_sub(&reserved, allocation.mapped);
}
