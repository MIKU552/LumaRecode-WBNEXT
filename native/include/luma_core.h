/*
 * LumaRecode shared core — C ABI, revision 1.
 *
 * Threading: calls are independent and may run on any thread. No global state
 * is mutated except the panic hook, and the core keeps no per-call state.
 *
 * Ownership:
 *   - every `char *` returned by luma_call_json must be freed with
 *     luma_string_free();
 *   - luma_core_version() returns static storage and must NOT be freed;
 *   - input pointers are borrowed for the call only.
 *
 * Errors: a call always returns a JSON envelope, never NULL on a rule failure:
 *   {"ok":true,"data":{...}}
 *   {"ok":false,"error":{"code":"policy.hdr_depth","detail":null}}
 * NULL means allocation failed; treat it as a fatal host error.
 *
 * The envelope transport is the revision-1 contract for planning calls. Frame
 * progress events stay outside it: those move to typed structs with explicit
 * sequence numbers before the first release build.
 */
#ifndef LUMARECODE_LUMA_CORE_H
#define LUMARECODE_LUMA_CORE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define LUMA_CALL __cdecl
#else
#define LUMA_CALL
#endif

/* ABI revision implemented by this header. */
#define LUMA_ABI_VERSION 1u

/* ABI revision reported by the loaded library. Compare before use. */
uint32_t LUMA_CALL luma_abi_version(void);

/* Crate version, e.g. "0.1.0". Static storage; do not free. */
const char *LUMA_CALL luma_core_version(void);

/*
 * Runs one operation.
 *
 * op:      NUL-terminated UTF-8 operation name (see below).
 * request: NUL-terminated UTF-8 JSON object; may be "{}" when unused.
 *
 * Returns an owned UTF-8 JSON envelope, or NULL when allocation failed.
 */
char *LUMA_CALL luma_call_json(const char *op, const char *request);

/* Releases a string returned by luma_call_json. NULL is ignored. */
void LUMA_CALL luma_string_free(char *value);

/*
 * Operations (request -> data):
 *
 *   core.info           {} -> {abi, core_version, target_arch, target_os, operations}
 *   policy.describe     {source, options?} -> {source, choices, audio}
 *   policy.resolve      {source, options?} -> {sampling, color_range, bit_depth, ...}
 *   policy.validate     {source, options?} -> same shape, after full validation
 *   timeline.resolve    {frames, time_base, packet_indexed?, range?} -> {count, duration, window}
 *   presets.resolve     {document, purpose, codec, variant?} -> preset + audio default
 *   presets.validate    {document} -> {count, presets}
 *   capability.select   {devices, requirement} -> {device_id, backend, score, reasons, fallback}
 *   capability.summary  {devices, codec} -> {backends, max_depth, samplings, hdr, dynamic_hdr}
 *   plan.build          {source, options?, backend?, frame_window?, motion_photo?, preview_size?}
 *                       -> {resolved, decode, transport, steps, notes}
 *
 * Frames never cross this boundary: the host keeps decoded images and passes
 * only timestamps, sizes and identifiers.
 *
 * Stable error codes are grouped by prefix:
 *   options.*  rejected request value
 *   policy.*   source/encoder format limit
 *   audio.*    audio eligibility
 *   transform.*, dvhdr.*, raw.*  color and dynamic HDR compatibility
 *   timeline.*, source.*  timestamps and index
 *   preset.*, capability.*, plan.*  tables, devices, planning
 *   ffi.*      boundary misuse
 */

#ifdef __cplusplus
}
#endif

#endif /* LUMARECODE_LUMA_CORE_H */
