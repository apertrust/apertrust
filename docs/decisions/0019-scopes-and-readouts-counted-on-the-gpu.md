# 0019. Scopes and readouts counted on the GPU

## Context

The Scopes section shows what the viewport's frame holds (a histogram, a waveform, an RGB parade, a vectorscope, what clips), and some module panels draw data the engine computes during rendering, the tone equalizer's mask histogram first. Whole-image results stay on the GPU ([0015](0015-whole-image-passes.md)), the engine thread never waits on the GPU ([0017](0017-threads-and-the-engines-session.md)), and modules have no engine types ([0028](0028-modules-bound-to-operations.md)). How are these counted and handed to a client?

## Decision

**Scopes: each viewport frame counted after it, outside the plan, once a client asks.**

- `Command::Scopes(Option<Graphs>)`: `None` until a client first asks, so a client without scopes pays nothing; `Some(Graphs { histogram, waveform, vectorscope })` the three totals (pixels counted, those with any channel at or below 0, those with any at or above 1) and the graphs named. Asking for what the frame shown was not counted for counts it again at once, without rendering.
- One pass of the engine's own kernel, `crates/engine/slang/scopes.slang`, over the frame in its display buffer, after colorout and before any rounding, a uniform flag per graph, so the frame is read once. Integer atomics in workgroup memory, then added to the counts buffer; the larger waveform and vectorscope straight into it.
- Bins `⌊v × 256⌋` limited to the ends, exact on every GPU. Luminance is Rec. 709's of the frame's linear light read as sRGB, encoded again with sRGB's curve, a grey its own value.
- The vectorscope is darktable's default: CIE 1976 u\*v\* against D50 through sRGB, its logarithmic scale, 128 × 128 cells, each with darktable's colour, turned as darktable shows it.
- Submitted on its own after the frame's copy, so a scopes failure never costs the frame ([0018](0018-one-submit-gate-for-the-engine-and-the-window.md)).
- **Readback:** two staging buffers in turn, each busy until its reading has been read; while both are busy the counts wait in the counts buffer, each frame's replacing the last's, so the last frame's reading always arrives. The engine thread polls between commands and never waits for a mapping.
- `Event::Scopes(Reading)`, plain data: the totals, each graph's `u32` counts, the frame's size, the GPU time and latency, and `through`, which `Frame::through` also carries, so a client pairs a reading with its frame.
- A failure turns the scopes off for the session, reported once (`SessionError::Scopes`); the frame is published all the same.

**Readouts** (planned): what an operation measures of the photo for its module's panel, as darktable's GUI computes from its preview pipe.

- The operation declares them: `Manifest::readouts`; `Prepared::readouts`, a `ReadoutPass` each, one pass of its kernel binding its counts buffer as `Resource::Readout`, never among `Prepared::passes`, so no export, tile, golden case or derivative sees it; `Operation::readout(id, words)`, the counts decoded on the CPU into `ReadoutValue::Histogram`.
- Planned in the viewport only; dispatched while a client asks for it by module and its key, a whole-image result's, has changed, so a band drag counts nothing.
- Read back through the scopes' staging pair, one helper both use; a failure turns readouts off, never the frame (`SessionError::Readouts`).
- `Command::Readouts(Vec<ReadoutId { module, readout }>)`, by module, never by operation; `Event::Readout(ReadoutReading { module, readout, through, value })`, `None` while the module's operation is not planned.
- The tone equalizer's `mask_histogram` first: darktable's 512 bins over −10 to +6 EV, sent as its 256 over −8 to 0 EV, the pixels and the 5th and 95th percentiles. RGB levels' and RGB curve's input histograms follow.

## Options considered

- A whole-image pass in the plan for the scopes: its result would still need this readback.
- Counting in colorout's kernel: ties every operation that ends the pipe to the scopes.
- The totals counted for every client: paid every frame with nothing to show.
- Bins on the 8-bit codes, `round(v × 255)`: edges depend on each GPU's rounding.
- One staging buffer, or waiting for the mapping: drops readings during a drag, or blocks the engine thread.
- A readout as an image the engine counts: 15 MB more at 2400×1600 and a write on every render; the mask read back and binned on the CPU; planning the pass only while asked, a rebuild per ask.

## Consequences

- On the M4 at 2400×1600 the count takes 0.64 to 0.67 ms of GPU time with the totals or histogram, 1.25 ms with the waveform.
- Open: scopes and readouts of a zoomed viewport, which need a small whole-image pipe; a readout with an argument (the cursor); how a reading drives an Auto. A reading over an area is a picker's, the engine's own reduction rather than an operation's readout, read back through the same staging pair ([0079](0079-colour-pickers-one-api-for-modules-and-the-colour-picker.md)), which also restricts the scopes' count to the Colour picker's selection on request.
