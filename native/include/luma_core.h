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
 * The envelope transport is the revision-1 contract for planning calls.
 * Progress events are modelled in the core (luma-core's ProgressEvent: task id,
 * sequence number, step index, fraction) but are not on this surface yet; the
 * callback entry point is the remaining half of the plan's section 3.3.
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
 *   core.info            {} -> {abi, core_version, target_arch, target_os, operations}
 *   policy.describe      {source, options?} -> {source, choices, audio}
 *   policy.resolve       {source, options?} -> {sampling, color_range, bit_depth, ...}
 *   policy.validate      {source, options?} -> same shape, after full validation
 *   timeline.resolve     {frames, time_base, packet_indexed?, range?}
 *                        -> {count, duration, packet_indexed, window}
 *   presets.audio        {document, preset} -> {name, encoder, bitrate_kbps, channels,
 *                        sample_rate, vbr, application, compression_level}
 *   presets.resolve      {document, purpose, codec, variant?} -> preset + audio default
 *   presets.validate     {document} -> {count, presets}
 *   capability.select    {devices, requirement}
 *                        -> {device_id, device_name, backend, score, reasons, fallback}
 *   capability.summary   {devices, codec}
 *                        -> {backends, max_depth, samplings, hdr, dynamic_hdr}
 *   plan.build           {source, options?, backend?, frame_window?, motion_photo?,
 *                        preview_size?} -> {resolved, decode, transport, steps, notes}
 *   probe.summarize      {document}
 *                        -> {width, height, rotation, kind, pixel_format, depth,
 *                            dolby_vision, hdr10_plus, audio, source, video_index,
 *                            frame_rate, side_data, source_range}
 *                        `source` is the normalized description `policy.*` accepts,
 *                        so a host never reads the probe document twice.
 *   capability.backend   {encoder} -> {backend}, null when the name is unknown
 *   capability.supports  {backend, codec, depth, sampling} -> {supported}
 *   cache.plan           {entries, current?, policy}
 *                        -> {decisions, kept_bytes, reclaimable_bytes}
 *   capability.detect    {encoders} -> {backends: [{backend, availability, encoders}]}
 *   capability.parameters {backend, codec} -> {parameters: [{name, kind, choices}]}
 *   plan.input_map       {input, video_index, audio_map, preserve_metadata,
 *                        preserve_chapters} -> {arguments}
 *   plan.color_args      {primaries?, transfer?, space?, range?, suppress_dv5_tags?}
 *                        -> {arguments}
 *   plan.encoder_args    {encoder, preset, quality, share, codec, depth, gop,
 *                        auto_low_power, timing_compatibility, async_depth,
 *                        pixel_format} -> {arguments}
 *   plan.parameter_string {values, appended?} -> {text}
 *   plan.filter_chain    {filters} -> {text}
 *   plan.gop             {frame_rate, share, motion_photo?} -> {frames}
 *   plan.hdr_metadata    {codec, output, source_kind?, preserve_dynamic?, dolby_vision_profile?,
 *                        hdr10_plus?, side_data?} -> {parameters, appended}
 *   plan.color_filters   {primaries?, transfer?, space?, range?, conversion_range, pixel_format,
 *                        purpose, depth, preserve_dynamic?} -> {filters}
 *   plan.render_filters  {log?, tone_map?, dolby_render?, drop_dynamic_metadata?, vpp?}
 *                        -> {log, tone_map, dolby_render, dropped_metadata, vpp}
 *   plan.color_tags      {kind, output, rgb?, dolby_profile5?, source_range, output_range,
 *                        primaries?, transfer?, space?} -> {source, target, source_filter}
 *   plan.log_profile     {profile} -> {profile, file, matrix, range}
 *   execution.run        {task_id?, commands:[{program, arguments}]} -> {ok, stopped_early,
 *                        outcomes}
 *
 * Progress: luma_set_progress_sink(callback, user_data) registers a sink that
 * execution.run calls once per finished step; pass a null callback to clear it.
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
