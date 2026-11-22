# Native callback budget and transport handoff

Native processing retains a 256-frame DSP quantum and 64-slot owned shared-memory queues. The queue capacity is independent of the declared transport delay. The default bridge budget remains 9472 source frames for clients that do not negotiate a physical callback bound.

The desktop negotiates the complete physical callback bound before starting a driver. A reported fixed buffer is enforced exactly; an unknown bound retains the global 8192-frame limit. The silent clock uses its actual 10 ms block. Larger callbacks are rejected in full before advancing PCM, automation or transport, then the existing device worker reports the error and changes to the explicit silent clock.

## Delay calculation

With engine rate `S`, device rate `D`, and physical callback bound `N`, the source burst is:

```text
source_burst = ceil(N × S / D) + 128 + 16
transport   = ceil((source_burst + 255) / 256) × 256
```

The 128 frames cover the output resampler's prefetch block; 16 cover its initial filter lookahead. The 255 frames cover a native quantum partially filled by the preceding callback. Bounds use whole-callback scheduling: the child process need not run until the entire physical callback has returned.

| Output | Source burst | Transport | At 48 kHz |
| --- | ---: | ---: | ---: |
| Silent clock, 480 frames | 624 | 1024 | 21.333 ms |
| Fixed 1024 frames, 48 kHz | 1168 | 1536 | 32 ms |
| Fixed 1024 frames, 44.1 kHz | 1259 | 1536 | 32 ms |
| Unknown 8192-frame bound, 44.1 kHz | 9061 | 9472 | 197.333 ms |

These are per-instance IPC transport delays. Plugin algorithmic delay and device/output conversion delay are additional. Realtime DSP remains at 48 kHz; the offline host has no IPC pipeline delay and separately supports its admitted export rates.

## Device, graph and timeline generation

1. The device worker retires the previous callback owner, configures the new output adapter and exact buffer bound, then publishes a token containing a generation and source burst **before** starting the driver.
2. The desktop registers its hosted-processor contract with `Control::require_callback_budget`. A registered callback with an obsolete timeline token emits silence and consumes pending graph/timeline commands without advancing DSP, MIDI events, conversion phase or the material position.
3. The existing desktop transport hold captures user play intent and position. Retained handles receive the new transport target; their host process, DSP state, shared ring and input sequence are retained. Chain padding, sidechain alignment and native schedules are rebuilt from the same budget.
4. `Control::set_timeline_for_callback_budget` tags a prepared timeline with its captured token and the latest submitted chain revision. A matching device token cannot overtake a backpressured PDC remap.
5. Only a matching timeline **and** applied chain revision make the callback ready. The desktop's existing chain reclamation and timeline receipts then restore the held transport. Stale background preparations cannot acknowledge a newer device generation.
6. A retained bridge changes its local delay at the next admitted processing boundary and warms its wet output for the new delay. Its source/event sequence is not reset.

Headless and offline engine clients are unregistered by default and retain their existing behavior. The public serialized project/timeline format does not acquire a device-specific field; tokens live only in the private runtime swap.

## Fault evidence and output semantics

Event-admission failure, a dead/stale host, and a full/unavailable native input queue remain separate first-failure snapshots. The callback writes fixed-size atomics only. The control/UI thread formats the plugin name, native sequence, input/output sequence, event count and host status. An event-containing rejected input publication still stops playback; events are never dropped to conceal a throughput problem.

A missed wet output deadline increments the existing xrun counter and outputs silence for that wet interval. It cannot expose unprocessed audio at the wrong compensation position. Explicit user bypass remains a separate direct path while continuing native DSP/event time.

The advanced demo failure observed with an unoptimized host was a real queue backlog: Limiter input 126, output 62, one event, host status OK. This is distinct from callback-budget admission. Production throughput must be checked with the release host and the same fixture/quality settings.

## Regression coverage

- Actual fixed-buffer rejection below the global 8192-frame maximum, including numerical and rate-converted outputs.
- Device budget publication before the first new-driver callback and fallback budget publication.
- Stale token rejection, including returning to an earlier frame bound with a newer generation.
- Resampler prefetch/phase hold without material-clock movement.
- A matching timeline blocked behind a full graph-retirement queue.
- Frozen-child callback bursts at 480/1024/8192 frames, 44.1/48 kHz and partial native quanta.
- Rebudgeting the same bridge/state/ring through short → long → short transport while preserving 72 exact native events.
- Phase alignment of retained one- and two-insert chains with newly prepared padding.
- Previously valid wet PCM followed by two missed deadlines, then explicit bypass.
- Headless/offline behavior without registration.
