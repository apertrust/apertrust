# 0031. Views and curves of the same stored values

## Context

darktable draws some stored values twice: White balance's coefficients as temperature and tint, colour calibration's illuminant as hue and chroma, the tone equalizer's bands as sliders and as a curve. Some of its controls write others to keep one quantity consistent. And a curve row must draw the curve its operation renders, not straight lines between points. How does a panel drawn from definitions do this with nothing stored twice and no maths in the app?

## Decision

**A lossless view is a second row** over the same stored values: an exact, invertible change of coordinates the module gives as pure methods of `Module`, `view` (the coordinates from the stored values) and `unview` (back).

- A coordinate is addressed like a control: `SetControl`, `ResetControl` and `Keep` take it, the session holding the view's other coordinates through a gesture (`edit::Held`) and reporting every view's coordinates with the values (`CoordinateState`).
- Nothing is stored under a coordinate's id. Marks and reset belong to the stored controls; once every coordinate is back at its default, the stored values are reset exactly.
- A view may have fewer coordinates than controls (temperature and tint over three coefficients), writing the controls as darktable's GUI does, normalised, and may read the camera through the photo's image context.
- While a change is pending, the app shows the view's controls and coordinates as the session will, through the module's own `view` and `unview`.
- Built: White balance's temperature and tint, Colour calibration's hue and chroma ([0027](0027-both-of-darktables-white-balances.md)). The colour equalizer's curves come with their widget.

**The tone equalizer's graph** shows the nine bands themselves on their own axes, the identity view: a custom row naming the band controls (`Widget::Equalizer`), with no coordinates of its own. A drag sends `SetControl` on the bands and its release ends the gesture on each band moved, so history merges a drag into one step. `Definition::check` allows `Widget::Equalizer` alone as such a second row, so a control named twice by mistake is still refused.

**Curves drawn as rendered: `Module::curve(control, values)`**, provided as `None` (straight lines), returns the curve a curve row or band graph draws, from the module's own values, pending ones included, so the curve follows a drag with no round trip. It runs on the CPU to draw, never to process a pixel. The tone equalizer's curve comes from the fit `apertrust-color` shares between the operation's setup and the graph (`tone_equalizer`), `None` where the fit fails, drawn in the warning colour; a drag settles the bands on it after each step, as darktable's does. `Module::curve_width(control, values)` gives the width σ of its Gaussians, how far a drag reaches, darktable's √2 by default.

**A curve control stores only the points between its ends**, (0, 0) and (1, 1), which the binding adds: RGB curve's up to 18 points, `rgbcurve`'s 20 nodes less the two ends, x at least 0.0025 apart and as far from the ends. RGB curve draws darktable's graph, with its compensate middle grey and graph scale.

**Follow writes: `Module::follow`.** Where darktable's GUI writes other parameters with a control to keep one quantity consistent, the module gives those writes, pure, and the session applies them in the same change and history step (`Edit::follow`), never rewriting the control the command set; a wrong one refuses the change whole (`EditError::FollowFailed`). Controls set the same quantity last-wins, as two views of one value do. A preset row's option takes none: it writes every control itself (`Module::preset`). darktable's session memory is not mirrored.

## Options considered

- The renderer converting stored values for a view: leaves scripts and remote clients without temperature and tint.
- Storing a view's coordinates: stores the light twice.
- The tone equalizer's graph as a view whose nine coordinates are the bands: nine coordinate ids beside nine controls of the same values.
- The session sampling the curve and reporting it: a pass and a readback per change, lagging a drag.
- A declarative interpolation in the definition, evaluated by the app: moves the operation's maths into the app.
- Storing a curve's ends with its points: stores them twice or holds a value nothing may change.
- Every control independent: a temperature move would keep "as shot to reference", a state darktable never reaches.

## Consequences

- A remote client cannot call `curve`, `view` or `follow`; it gets them with the description of the modules for remote clients, still to design.
- The app calls every module method guarded, so a panic draws no curve and moves nothing.
